//! The call tree: who called whom in a run, and what each of them cost.
//!
//! The emulator has two halves of the seam and each already keeps its own record.
//! [`crate::hle::Hle::calls`] is every guest-to-host dispatch (`IDisplay::BitBlt` and its
//! seventy-odd thousand friends), and [`crate::hle::Hle::downcalls`] is every branch the host made
//! back into the module (the applet's own event handler, mostly). Those two histories are kept
//! *flat* and *capped*: `Hle::HISTORY` is 4096 records and half of them are dropped when a run
//! passes that, which is the price of not letting a shipped game grow the process until Windows
//! fast-fails it. So they answer "what happened lately" and they cannot answer "what did this run
//! spend itself on".
//!
//! This module answers the second question by folding as it goes rather than by keeping a
//! sequence: [`Tree::enter`] interns a child of the current node and [`Tree::exit`] adds the delta
//! to it, so the memory a run needs is one node per *distinct* (parent, callee) pair rather than
//! one record per call. That is also exactly the shape a flame graph wants — a flame graph *is*
//! the folded tree — so there is one structure behind both of the things a person asks for:
//!
//! - the **call tree**: indentation, so "what did this frame do" reads top-down;
//! - the **flame graph**: the same rows with a width, so "which of them was most of it" reads at
//!   a glance.
//!
//! # Two ways to weigh a node, and why both are kept
//!
//! `steps` is the guest instruction count at the moment of the reading, and it is the honest,
//! reproducible weight: it does not move between two runs of the same pack, and the repository's
//! whole measurement discipline rests on that. What it cannot express is
//! the host's *own* work: while a Rust handler runs, the guest retires nothing, so every host
//! method would be zero wide and the flame graph would say the emulator spends all its time in the
//! guest. That is what `nanos` is for, and it is why the same node carries both.
//!
//! Nanoseconds are wall clock, so they carry the machine's noise (±40 % between rounds of the same
//! run) and they cannot be asserted in a test.
//! They are also not free: taking one costs an `Instant::now()`, so an armed run is measurably
//! slower than a bare one. Both facts are why the whole module is **off by default** — an
//! unarmed run pays one `bool` test per enter/exit and nothing else.
//!
//! # What is deliberately not here
//!
//! A function-level tree *inside* the module. That would need the module's own call graph, which
//! nobody wrote down: the ARM code has no frame pointer discipline to walk and a `.mod` carries no
//! symbols, so the only ways to recover it are heuristics (BL/BLX plus LR) that would print a tree
//! that is sometimes wrong. A wrong tree is worse than no tree, because it is read as evidence.

use std::collections::HashMap;
use std::time::Instant;

use crate::abi;

/// How deep a tree may go before it starts refusing frames.
///
/// The runtime's own nesting is capped by `MAX_DOWNCALL_DEPTH` (host stack, not this), and a guest
/// that recurses into us through it can be deeper than a reader can see. Past this point a frame is
/// counted as dropped rather than folded, and the report says so.
pub const MAX_DEPTH: usize = 128;

/// How many distinct nodes one run may build.
///
/// A shipped run reaches a few thousand distinct (parent, callee) pairs — there are 2132 methods in
/// the ABI table and a pack uses a small fraction of them from a handful of parents. This ceiling
/// exists for the run that does *not* look like that (a pack computing a dispatch address), and it
/// is reported rather than silently absorbing the overflow.
pub const MAX_NODES: usize = 32_768;

/// The index standing for "no node": a frame past the depth ceiling, or a callee past the node
/// ceiling. It is pushed and popped like any other frame so the pairing stays right; it just does
/// not accumulate anything.
const NO_NODE: u32 = u32::MAX;

/// The root's index, which is always 0 and is never popped.
const ROOT: u32 = 0;

/// What a node of the tree stands for.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Who {
    /// A stretch of the host's own work that the host named: the frame loop, the screen upload, a
    /// run. These are the only rows whose width comes from somewhere other than the guest.
    Span(&'static str),
    /// A guest-to-host dispatch: one `SVC`, i.e. one interface method or helper. Folded by the
    /// method id the ABI table assigned it, which is what makes it cheap enough to record on every
    /// call — the name is put back on at report time by
    /// [`abi::label`](crate::abi::label).
    Dispatch(u32),
    /// A host-to-guest branch: the runtime calling one of the module's own vtable slots. Folded by
    /// the address of the slot, because the module's code has no names to fold by.
    Downcall(u32),
}

impl Who {
    /// The kind, for a report that wants to group or colour by it.
    pub fn kind(&self) -> &'static str {
        match self {
            Who::Span(_) => "host",
            Who::Dispatch(_) => "call",
            Who::Downcall(_) => "down",
        }
    }

    /// What to call this node in a report.
    ///
    /// The one downcall address that has a name is the applet's own event handler, because "did my
    /// key reach the pack" is the question a person at a keyboard is asking and this is the only
    /// row that answers it. Every other slot is the module's private code and keeps its number —
    /// the number is the finding when there is no name, the same rule the call log follows.
    ///
    /// The event and key words deliberately do *not* go into the name: a name carrying them would
    /// make one node per key press, which turns a folded tree into the very thing this module
    /// exists to avoid.
    pub fn name(&self, handle_event: u32) -> String {
        match self {
            Who::Span(s) => (*s).to_string(),
            Who::Dispatch(id) => abi::label(*id).to_string(),
            Who::Downcall(addr) if *addr == handle_event => "IApplet::HandleEvent".to_string(),
            Who::Downcall(addr) => format!("guest {addr:#010x}"),
        }
    }
}

/// The two weights a node carries: guest instructions, and the host's own wall clock.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Cost {
    pub steps: u64,
    pub nanos: u64,
}

impl Cost {
    pub const ZERO: Cost = Cost { steps: 0, nanos: 0 };

    /// The milliseconds these nanoseconds are, for a report.
    pub fn ms(&self) -> f64 {
        self.nanos as f64 / 1_000_000.0
    }
}

/// Which weight a report is drawn by.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weight {
    /// Guest instructions retired. Reproducible; the width of the emulator's work.
    Steps,
    /// Wall clock nanoseconds. Not reproducible, not free, but it is the only weight the host's own
    /// methods have.
    Nanos,
}

impl Weight {
    pub fn of(&self, c: Cost) -> u64 {
        match self {
            Weight::Steps => c.steps,
            Weight::Nanos => c.nanos,
        }
    }
}

/// One frame of the tree: a callee, and everything that happened under it.
pub struct Node {
    pub who: Who,
    /// How many times this callee was entered from this parent.
    pub calls: u64,
    /// From this frame opening to this frame closing.
    pub total: Cost,
    /// What the children that closed inside this frame accounted for. Kept instead of subtracting
    /// from an `own` field as they close, because a child always closes *before* its parent: a
    /// subtraction at that moment would come off a `total` that has not been added yet, and a
    /// saturating subtraction of a number from zero is how that mistake disappears quietly. Added
    /// up here, the answer does not depend on the order anything closed in.
    kids_cost: Cost,
    kids: HashMap<Who, u32>,
    /// Kid indices in the order they were first seen, so a report of equal-width children does not
    /// reorder itself between two runs (a `HashMap`'s order is not a fact about the run).
    order: Vec<u32>,
}

impl Node {
    fn new(who: Who) -> Self {
        Node {
            who,
            calls: 0,
            total: Cost::ZERO,
            kids_cost: Cost::ZERO,
            kids: HashMap::new(),
            order: Vec::new(),
        }
    }
}

/// A frame in flight: the node it opened, and the reading when it opened.
#[derive(Clone, Copy)]
struct Frame {
    node: u32,
    entered: Cost,
}

/// The folded call tree.
pub struct Tree {
    nodes: Vec<Node>,
    /// The path from the root to the frame in flight. Never empty: index 0 is the root.
    stack: Vec<Frame>,
    /// Frames refused by [`MAX_DEPTH`] or [`MAX_NODES`], so a report can say the tree is short
    /// rather than let a reader believe it is complete.
    dropped: u64,
    /// How many enter/exit pairs went through this tree at all, dropped ones included.
    entered: u64,
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

impl Tree {
    /// A tree holding nothing but its root.
    pub fn new() -> Self {
        let mut root = Node::new(Who::Span("run"));
        root.calls = 1;
        Tree {
            nodes: vec![root],
            stack: vec![Frame {
                node: ROOT,
                entered: Cost::ZERO,
            }],
            dropped: 0,
            entered: 0,
        }
    }

    /// Forget every node but the root, keeping the mount points.
    pub fn clear(&mut self) {
        let mut root = Node::new(Who::Span("run"));
        root.calls = 1;
        self.nodes.clear();
        self.nodes.push(root);
        self.stack.truncate(1);
        self.dropped = 0;
        self.entered = 0;
    }

    /// Open a frame under whatever is running right now.
    pub fn enter(&mut self, who: Who, at: Cost) {
        self.entered += 1;
        let parent = self.stack.last().map_or(ROOT, |f| f.node);
        let node = if parent == NO_NODE || self.stack.len() > MAX_DEPTH {
            NO_NODE
        } else {
            self.intern(parent, who)
        };
        // Both refusals are counted here rather than inside `intern`, so that neither can be the
        // kind of loss nobody can see: a tree that stopped growing must say it stopped growing.
        if node == NO_NODE {
            self.dropped += 1;
        } else {
            self.nodes[node as usize].calls += 1;
        }
        self.stack.push(Frame { node, entered: at });
    }

    /// Close the frame that is in flight and charge it with what it cost.
    ///
    /// The delta goes to the closing frame's own `total` and to its parent's `kids_cost`, which is
    /// what makes `own` (total minus children) independent of the order anything closed in. It is
    /// the parent that needs the entry, not the child that needs to subtract from it: see
    /// [`Node::kids_cost`]. Everything is saturating: a reading that appears to go backwards (a
    /// clock that was reset, a counter restored by a nested call) must not wrap into a number a
    /// reader would take as a finding.
    pub fn exit(&mut self, at: Cost) {
        let Some(frame) = self.stack.pop() else {
            return;
        };
        if frame.node == NO_NODE {
            return;
        }
        let delta = Cost {
            steps: at.steps.saturating_sub(frame.entered.steps),
            nanos: at.nanos.saturating_sub(frame.entered.nanos),
        };
        let node = &mut self.nodes[frame.node as usize];
        node.total.steps = node.total.steps.saturating_add(delta.steps);
        node.total.nanos = node.total.nanos.saturating_add(delta.nanos);
        if let Some(parent) = self.stack.last().copied() {
            if parent.node != NO_NODE {
                let p = &mut self.nodes[parent.node as usize];
                p.kids_cost.steps = p.kids_cost.steps.saturating_add(delta.steps);
                p.kids_cost.nanos = p.kids_cost.nanos.saturating_add(delta.nanos);
            }
        }
    }

    /// The node for `who` under `parent`, built on first sight.
    fn intern(&mut self, parent: u32, who: Who) -> u32 {
        if let Some(&i) = self.nodes[parent as usize].kids.get(&who) {
            return i;
        }
        if self.nodes.len() >= MAX_NODES {
            return NO_NODE;
        }
        let i = self.nodes.len() as u32;
        self.nodes.push(Node::new(who));
        let p = &mut self.nodes[parent as usize];
        p.kids.insert(who, i);
        p.order.push(i);
        i
    }

    pub fn root(&self) -> &Node {
        &self.nodes[ROOT as usize]
    }

    /// What a node cost, root included.
    ///
    /// The root is the one node that never closes — nothing enters it and nothing exits it, because
    /// it stands for the run itself — so its own `total` would stay zero and a report that divided
    /// by it would divide by zero. Its width is therefore defined as the sum of what it directly
    /// holds, which is the same statement as "everything the run did" and is the number a reader
    /// means when they ask how wide the run was.
    pub fn total(&self, i: u32) -> Cost {
        if i != ROOT {
            return self.nodes[i as usize].total;
        }
        let mut sum = Cost::ZERO;
        for &k in &self.nodes[ROOT as usize].order {
            let t = self.nodes[k as usize].total;
            sum.steps = sum.steps.saturating_add(t.steps);
            sum.nanos = sum.nanos.saturating_add(t.nanos);
        }
        sum
    }

    /// What a node cost on its own: its total minus what closed under it.
    ///
    /// This is the number that answers "who is actually spending it" — the widest `total` in a tree
    /// is almost always the root, and the widest `own` is the finding. The root's own is zero: its
    /// total is defined as the sum of what it holds, and that is exactly what its children reported.
    pub fn own(&self, i: u32) -> Cost {
        let total = self.total(i);
        let kids = self.nodes[i as usize].kids_cost;
        Cost {
            steps: total.steps.saturating_sub(kids.steps),
            nanos: total.nanos.saturating_sub(kids.nanos),
        }
    }

    /// How many times a node was entered.
    pub fn calls(&self, i: u32) -> u64 {
        self.nodes[i as usize].calls
    }

    pub fn node(&self, i: u32) -> &Node {
        &self.nodes[i as usize]
    }

    pub fn nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Frames this tree refused. A non-zero number here means the report is a lower bound.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Enter/exit pairs seen, dropped ones included. Zero means nothing was recorded, which is a
    /// different statement from "nothing happened".
    pub fn entered(&self) -> u64 {
        self.entered
    }

    /// Frames open right now: zero between turns, non-zero only *inside* one.
    ///
    /// A reader looking after a run should find zero here. Anything else means a frame was entered
    /// and never closed — a path that returned early through a `?`, or a test that forgot one half of
    /// a pair — and it is worth having as a number of its own because a tree missing an exit still
    /// looks plausible: the frame that should have closed takes the *next* exit instead, and its
    /// width comes out as whatever was between them.
    pub fn depth(&self) -> usize {
        self.stack.len() - 1
    }

    /// Whether anything beyond the root was folded.
    pub fn is_empty(&self) -> bool {
        self.nodes.len() <= 1
    }

    /// This node's children, widest first under `weight`.
    ///
    /// Width decides the order because a flame graph is read for its widest row; ties keep the
    /// order they were first seen in, which is a property of this run rather than of a hash table.
    pub fn children(&self, i: u32, weight: Weight) -> Vec<u32> {
        let mut kids = self.nodes[i as usize].order.clone();
        kids.sort_by(|a, b| {
            let (x, y) = (self.total(*a), self.total(*b));
            weight
                .of(y)
                .cmp(&weight.of(x))
                .then(
                    self.nodes[*a as usize]
                        .calls
                        .cmp(&self.nodes[*b as usize].calls)
                        .reverse(),
                )
                .then(a.cmp(b))
        });
        kids
    }
}

/// One row of a report: a node, how deep it sits, and how wide it is.
pub struct Line {
    pub node: u32,
    pub depth: usize,
    /// This node's total, as a fraction of the root's — the bar.
    pub share: f64,
    /// This node's own cost, as a fraction of the root's.
    pub self_share: f64,
}

/// One row of a drawn tree: enough for a front end that cannot see the machine.
#[derive(Clone, Debug)]
pub struct FlameRow {
    pub depth: usize,
    /// Left edge as a fraction of the whole run, and the reason this is an icicle and not a list.
    pub start: f64,
    /// Width as a fraction of the whole run.
    pub share: f64,
    /// This row's own work, as a fraction of the row's own total — the number that says whether the
    /// width is the callee's or its children's.
    pub self_share: f64,
    pub name: String,
    /// `"host"`, `"call"` or `"down"`, so a drawing can colour them apart.
    pub kind: &'static str,
    pub calls: u64,
    pub ms: f64,
    pub steps: u64,
}

/// A tree ready to be drawn: the rows, and what a reader needs to know about the cutting.
#[derive(Clone, Debug, Default)]
pub struct Flame {
    pub rows: Vec<FlameRow>,
    /// Rows left out by depth or width. Non-zero means the picture is a lower bound, and a drawing
    /// that hides this turns a cut tree into a complete-looking one.
    pub elided: usize,
    pub nodes: usize,
    /// Enter/exit pairs folded. Zero means nothing was recorded at all, which is a different
    /// statement from "the run did nothing".
    pub folded: u64,
    pub dropped: u64,
    pub total_ms: f64,
    pub total_steps: u64,
    pub depth: usize,
    /// The depth this was cut at, so a report can say where it stopped rather than only that it did.
    pub cut_depth: usize,
}

/// How a report is cut down to something a person can read.
#[derive(Clone, Copy)]
pub struct Options {
    pub weight: Weight,
    /// Rows deeper than this are counted and not printed.
    pub max_depth: usize,
    /// Rows narrower than this fraction of the root are counted and not printed.
    pub min_share: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            // Wall clock by default: a flame graph is read for "where did the time go", and under
            // the instruction count every host method is zero wide — the guest retires nothing while
            // Rust runs — which would draw a graph that says all of it was the guest's own code.
            weight: Weight::Nanos,
            max_depth: 6,
            min_share: 0.005,
        }
    }
}

/// How many characters a row's bar is drawn in.
const BAR: usize = 24;

/// A row's bar: `#` for the share it holds, spaces for the rest.
fn bar(share: f64) -> String {
    let taken = (share.clamp(0.0, 1.0) * BAR as f64).round() as usize;
    format!("{}{}", "#".repeat(taken), " ".repeat(BAR - taken))
}

impl Tree {
    /// The tree as rows, in the order a table is read: each row, then everything under it, then its
    /// next sibling.
    ///
    /// **That order is not cosmetic**, which this got wrong: it used a LIFO stack, so `out` held the
    /// children in the right order while the *subtrees* were walked last-pushed-first — a node's
    /// descendants came out after its next sibling's row, and an indented table then said a callee
    /// belonged to the wrong parent. A call tree is read by its indentation, so a reader who trusts it
    /// was being lied to. The recursion is bounded by [`MAX_DEPTH`] and the tree by [`MAX_NODES`], so
    /// this cannot walk deeper than the tree can be.
    ///
    /// Elided rows are not silent: the returned count includes both what the caller's cuts dropped and
    /// what [`MAX_DEPTH`] did, so a report can say "… N narrower rows not shown" instead of ending like
    /// a complete one.
    pub fn lines(&self, opt: Options) -> (Vec<Line>, usize) {
        let mut out = Vec::new();
        let mut elided = 0;
        let root = opt.weight.of(self.total(ROOT));
        if root == 0 {
            return (out, 0);
        }
        out.push(Line {
            node: ROOT,
            depth: 0,
            share: 1.0,
            self_share: opt.weight.of(self.own(ROOT)) as f64 / root as f64,
        });
        self.walk_lines(ROOT, 0, opt, root, &mut out, &mut elided);
        (out, elided)
    }

    /// [`Tree::lines`]' recursion: one node's children in width order, each followed by its own.
    fn walk_lines(
        &self,
        i: u32,
        depth: usize,
        opt: Options,
        root: u64,
        out: &mut Vec<Line>,
        elided: &mut usize,
    ) {
        for kid in self.children(i, opt.weight) {
            let share = opt.weight.of(self.total(kid)) as f64 / root as f64;
            if depth + 1 > opt.max_depth || share < opt.min_share {
                *elided += 1;
                continue;
            }
            out.push(Line {
                node: kid,
                depth: depth + 1,
                share,
                self_share: opt.weight.of(self.own(kid)) as f64 / root as f64,
            });
            self.walk_lines(kid, depth + 1, opt, root, out, elided);
        }
    }

    /// A row's children count, for a caller deciding whether to recurse.
    pub fn child_count(&self, i: u32) -> usize {
        self.nodes[i as usize].order.len()
    }

    /// The tree as text: the command line's report.
    ///
    /// A thin wrapper over the one renderer ([`Flame::report`]) because "the run is over, print it" has
    /// a `Tree` in hand while a front end has rows. One renderer is the point: a saved tree and a
    /// printed one are the same text, and this repository has paid more than once for a contract
    /// written in one place and drawn in another.
    pub fn report(&self, handle_event: u32, opt: Options) -> String {
        self.flame(handle_event, opt)
            .report(opt.min_share, opt.weight)
    }

    /// The tree flattened into rows a front end can draw, without the machine.
    ///
    /// This exists because of the thread the machine lives on: a window and a game are on different
    /// threads, and the window may not borrow the `Tree` — so what crosses is a value, built here so
    /// that both front ends and the report agree about what a row is.
    ///
    /// `start` is the left edge as a fraction of the whole, and it is what makes an icicle rather
    /// than a list: a child begins where its parent's earlier children ended.
    ///
    /// **Width is therefore not a cut this can make.** [`Options::min_share`] is ignored here, and
    /// only [`Options::max_depth`] applies: a row dropped for being narrow would move every later
    /// sibling left, and the picture would then claim two things are in different places from where
    /// they are. A drawing that cannot fit a row should make it thin, not delete it.
    pub fn flame(&self, handle_event: u32, opt: Options) -> Flame {
        let opt = Options {
            min_share: 0.0,
            ..opt
        };
        let (lines, elided) = self.lines(opt);
        let mut rows = Vec::with_capacity(lines.len());
        // Where the next node at each depth begins. Depth first, widest child first — the order
        // `lines` already returns — is exactly the order an icicle puts same-depth nodes in, so one
        // cursor per depth is enough.
        let mut cursor: Vec<f64> = Vec::new();
        for l in &lines {
            while cursor.len() <= l.depth {
                cursor.push(0.0);
            }
            let start = cursor[l.depth];
            cursor[l.depth] = start + l.share;
            let node = self.node(l.node);
            let total = self.total(l.node);
            let own = self.own(l.node);
            // Of *this row's* width, how much was its own work — **not** of the run's. The two are easy
            // to mix up, and the difference is the question a flame graph is read for: 3 % of the run
            // and 100 % its own is a hot leaf; 3 % of the run and 1 % its own is a frame whose children
            // are holding everything.
            let self_share = match opt.weight.of(total) {
                0 => 0.0,
                whole => opt.weight.of(own) as f64 / whole as f64,
            };
            rows.push(FlameRow {
                depth: l.depth,
                start,
                share: l.share,
                self_share,
                name: node.who.name(handle_event),
                kind: node.who.kind(),
                calls: node.calls,
                ms: total.ms(),
                steps: total.steps,
            });
        }
        let depth = rows.iter().map(|r| r.depth).max().unwrap_or(0);
        let whole = self.total(ROOT);
        Flame {
            rows,
            elided,
            nodes: self.nodes.len(),
            folded: self.entered,
            dropped: self.dropped,
            total_ms: whole.ms(),
            total_steps: whole.steps,
            depth,
            cut_depth: opt.max_depth,
        }
    }
}

impl Flame {
    /// The tree as one person reads it: a row per frame, indented by depth, with a bar for width.
    ///
    /// **The one place this text is built.** The command line's report and the file the developer page
    /// saves are the same rendering over the same rows, so they cannot drift — this repository has paid
    /// more than once for a contract written in one place and drawn in another.
    ///
    /// Both weights are printed on every row even though only one draws the bar, because they answer
    /// different questions and comparing them is the point: a row that is wide in milliseconds and
    /// narrow in instructions is host code, and one that is the other way round is the guest.
    ///
    /// `min_share` cuts the **list** and nothing else — rows under it are counted and skipped, and the
    /// count is printed. It is a parameter rather than a rule because the two callers want different
    /// answers: a terminal is somebody reading, and a saved file is worth having whole.
    pub fn report(&self, min_share: f64, weight: Weight) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "call tree (drawn by {}): {} node(s), {} frame(s) folded, {} dropped\n\
             run: {} instr, {:.2} ms\n",
            match weight {
                Weight::Steps => "instructions",
                Weight::Nanos => "wall clock",
            },
            self.nodes,
            self.folded,
            self.dropped,
            self.total_steps,
            self.total_ms,
        ));
        let mut elided = self.elided;
        for row in &self.rows {
            if row.share < min_share {
                elided += 1;
                continue;
            }
            out.push_str(&format!(
                "{}{:>5.1}% {} {:>9.2} ms {:>12} instr {:>7}x {:>5.1}% own  {}{}\n",
                "  ".repeat(row.depth),
                row.share * 100.0,
                bar(row.share),
                row.ms,
                row.steps,
                row.calls,
                row.self_share * 100.0,
                row.name,
                if row.self_share >= 1.0 {
                    "   (all its own)"
                } else {
                    ""
                },
            ));
        }
        if elided > 0 {
            out.push_str(&format!(
                "… {elided} frame(s) narrower than {:.1}% or deeper than {} not shown\n",
                min_share * 100.0,
                self.cut_depth,
            ));
        }
        out
    }
}

/// The run's tree plus the switch that decides whether anything reaches it.
///
/// Off unless someone asks: this exists for the person debugging a pack's performance, and every
/// other run should pay nothing for it. The check is the first thing in [`Profile::enter`] and it
/// is one `bool` — the same shape the interpreter uses for watchpoints and sampling (`Mem::watch`
/// is empty, `Samples::every` is zero), for the same reason.
pub struct Profile {
    on: bool,
    epoch: Instant,
    tree: Tree,
}

impl Default for Profile {
    fn default() -> Self {
        Self::new()
    }
}

impl Profile {
    pub fn new() -> Self {
        Profile {
            on: false,
            epoch: Instant::now(),
            tree: Tree::new(),
        }
    }

    pub fn on(&self) -> bool {
        self.on
    }

    /// Turn recording on or off. Turning it on after a run has already gone leaves the tree as it
    /// was rather than starting over: what the page or the report shows is then "since the last
    /// reset", which is what someone flipping the switch mid-run means.
    pub fn set(&mut self, on: bool) {
        self.on = on;
    }

    /// Throw away what has been folded so far.
    pub fn clear(&mut self) {
        self.tree.clear();
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Nanoseconds since this profile was built — the clock the tree's `nanos` are in.
    pub fn now(&self) -> u64 {
        self.epoch.elapsed().as_nanos() as u64
    }

    /// Open a frame, reading the wall clock if — and only if — this profile is armed.
    pub fn enter(&mut self, who: Who, steps: u64) {
        if !self.on {
            return;
        }
        let at = Cost {
            steps,
            nanos: self.now(),
        };
        self.tree.enter(who, at);
    }

    /// Close the frame in flight.
    pub fn exit(&mut self, steps: u64) {
        if !self.on {
            return;
        }
        let at = Cost {
            steps,
            nanos: self.now(),
        };
        self.tree.exit(at);
    }
}

#[cfg(test)]
#[path = "test/profile.rs"]
mod tests;
