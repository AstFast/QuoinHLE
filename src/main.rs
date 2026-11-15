//! `quoin` — load a BREW `.mod` and run it against the emulated AEE runtime.
//!
//! Two shapes of run, in this order: bring the module up and call `AEEMod_Load`, then — with
//! `--start` — create its applet, deliver `EVT_APP_START`, and pump whatever scripted events
//! follow. A run that stops at "unimplemented IShell::SetTimer" is a useful result, not a failure:
//! it is the work list, derived from real guest code rather than guessed at.

use std::process::ExitCode;

use quoin::abi;
use quoin::audio::AudioBackend;
use quoin::cpu::{Samples, Trace, SAMPLE_CAP};
use quoin::device::DeviceProfile;
use quoin::hle::ArgText;
use quoin::runtime::{AcceptedClass, Dispatched, Outcome, PadInput, Runtime, ThreadState};
use quoin::{disasm, symbol};

const USAGE: &str = "\
QuoinHLE — BREW HLE emulator

  quoin <file.mod> [options]
  quoin --make-bbf FONT [--out FILE.bbf] [--size N] [--face-index N] [--half-widths N]
                                   convert a system TrueType/OpenType font into the bitmap container
                                   the guest reads. `--version 1` writes the variable-width layout
                                   (one row, each glyph's own advance; `wWidth` is u16, so small
                                   repertoires only); `--version 2` (default) writes the single-cell
                                   layout with 256 glyphs per row. A build-time tool, not a run.
  quoin                            open the game library: the packs under BREW_GAME_DIR, or under
                                   Game/ beside the working directory. Pick one to play it, and
                                   Esc goes back to the list.

  --dev              enable developer options. Diagnostics, tracing, network scripts, recording
                     and static inspection are refused without it.
  Developer options (require --dev):
                     --trace --sample --trace-limit --trace-range --trace-events --disasm --dump
                     --break --watch --watch-stop --watch-chain --coverage --flame --args --verbose
                     --info --color-depth --encoding --thumb-entry --bmp --dump-audio --dump-wav
                     --record-input --input-log --serve --net-proxy --net-script --scripts --script-writes

  --trace            print every retired guest instruction and every HLE dispatch
  --trace-limit N    stop printing after N instructions, and say so when it stops: a run that goes
                     wrong early fills a terminal faster than it can be read
  --trace-range LO,HI
                     print only the instructions of that span, which is the question that gets asked
                     when something branched somewhere unexpected. Each bound takes a number or a
                     region offset (`heap+0x…`, `stack-0x…`)
  --trace-events FILE
                     write every IApplet::HandleEvent delivery to FILE as TSV: virtual
                     milliseconds, event, wParam, dwParam, result. Flushed per event, so a run that
                     faults still leaves the sequence that led to it.
  --thumb-entry      the image begins in Thumb state (default: ARM)
  --device WxH       answer that screen size to the module, for a pack compiled for
                     another handset screen. A pack is built for one screen and lays its art out for
                     it, so on a different size it draws misaligned rather than failing — match the
                     pack. What was answered is printed at the start and in the window's title.
  --device-json FILE
                     read the handset from FILE instead of the built-in one: screen, keypad, identity
                     strings, storage, timers. Every field of the device profile has a key of the same
                     name, and a key left out keeps the built-in value, so a file with two lines is a
                     valid device. A FILE that does not exist is written there with the built-in
                     handset, which is how the template is made. `--device`, `--ram-mb`,
                     `--color-depth` and `--encoding` still override it.
  --save-device      write the profile back to the file --device-json named, after those flags have
                     applied. That is how a set of flags that described a handset becomes a file:
                     run once with the flags and this, then edit from there.
  --heap-mb N        give the guest heap an N MiB span above the arena base. Default 240, the
                     delivery's own. This is the ceiling `Sbrk` may grow to, so a pack that asks for
                     more than the default can still get it; `GetModuleMemStats`'s allowance follows.
  --heap-initial-kb N
                     how many KiB the heap's first free node covers. Default 256. It moves the point
                     where an allocation stops fitting and starts growing the heap, not the ceiling:
                     the `heap: … reached` line starts lower, and no address changes.
  --ram-mb N         answer N as `AEEDeviceInfo::dwRAM`, the total RAM a handset reports. Default 64.
                     This is the figure a pack plans against; it is not the span the heap can really
                     reach.
  --color-depth N    answer N as AEEDeviceInfo::nColorDepth, to see which code path a pack takes on it.
                     A DIAGNOSTIC, not a screen: the frame stays RGB565 and AEEBitmapInfo::nDepth keeps
                     saying 16, so the two answers stop agreeing and a pack that composites straight
                     into IDisplay::GetDestination's pBmp will lay out wrong. Wrong art there is the
                     pack showing you its branch, not this host losing pixels.
  --encoding N       answer N as AEEDeviceInfo::wEncoding (an AEE_ENC_*: 2 UTF8, 0xFC EUC_CN, 0xFD
                     S_JIS, 0xFF UNICODE). Cleanest device A/B here, because this host only publishes
                     the number — no text path of ours reads it, so only the pack's branches move.
  --run-for N        pump the dispatch loop for up to N milliseconds of virtual time after the
                     scripted input: queued events, then whatever the applet armed for itself,
                     until nothing is left to run or the window is spent
  --steps N          how many instructions the guest may retire *without reaching the host* before
                     the run says it is waiting on something that will not arrive (default 2000000,
                     or 32000000 for a pack being played — see GAME_STEP_LIMIT, which measures it
                     across the corpus). Reaching the host restarts the count, so this catches a
                     guest-only spin without calling a shipped game's expensive frame a hang.
  --steps-ceiling N  the total one guest function may retire however often it reaches the host
                     (default 1000000000 for a pack being played). This is what still gives a
                     scripted run an end when a loop is long *and* never finishes; a function that
                     hits it is reported as working-but-endless, not as stuck.
  --session-ceiling N
                     the total the WHOLE run may retire, across every function and every downcall
                     nested inside it, from the run's first instruction. Both bounds above restart,
                     so a pack that returns from each function it was given but never finishes its
                     task runs out of neither; this is the one that ends such a run and says where.
                     Off by default.
  --verbose          also print the volume: the HLE call log, every text run, every note, the list
                     of every audio source named. What comes out without it is what a run was asked
                     for — the counts, the one-line resource and audio summaries, and whatever a
                     watchpoint, --break, --disasm, --dump or --coverage wanted answered — and a run
                     that ends in a fault says so either way.
  --start CLS        create and start that applet once the module is loaded
  --start auto       ask the module which class id in its own image it builds an applet for,
                     and start that one.
                     A run that --break or --watch-stop asked for stopped during the load does NOT
                     start it: the report would otherwise be a machine from after the stop while its
                     first line still says「stopped」. The run says `--start skipped` when that happens.
  --event E[,WP[,DWP]]
                     post an event to the running applet (repeatable)
  --pointer KIND,X,Y post one pointer contact: KIND is `down`, `move`, `up`, or `tap` for the
                     whole press (repeatable). This is what a touchscreen pack waits on, and it is
                     NOT reachable through --event: EVT_POINTER_* carries its coordinates as a
                     guest-memory string, `x=00000078,y=000000fa,time=…,clkcnt=1`, whose address is
                     dwParam and whose length is wParam. Passing packed numbers there instead makes
                     the module parse an integer as text and fault inside its own hex decoder.
                     X and Y may be negative or hex, because a handset reports them signed.
  --key CODE         press and release one key, e.g. --key AVK_DOWN (repeatable)
  --hold MS          how long a --key holds the key down before releasing it, in virtual
                     milliseconds (default 150). Apps that poll which keys are currently held, as
                     a pack does, cannot see a press and a release that land in the same turn.
  --pad TARGET,STATE move one control of the virtual joystick this console answers through `IHID`
                     (repeatable, and queued in the same order as --key and --wait). A title
                     that reads its pad from `IHIDDevice` rather than from keys is only reachable
                     this way, and --key cannot stand in for it: such a pack asks what is *down*, so
                     the answer has to be state the host holds and not just an event it was sent.
                     TARGET is a button by the name this host publishes for it — BACK, START,
                     DPAD_UP, DPAD_LEFT, DPAD_DOWN, DPAD_RIGHT, BUTTON_1..BUTTON_4,
                     LEFT_SHOULDER_UPPER, RIGHT_SHOULDER_UPPER, LEFT_SHOULDER_LOWER,
                     RIGHT_SHOULDER_LOWER, LEFT_THUMBSTICK, RIGHT_THUMBSTICK — or its id 0..15, or an
                     axis: LEFT_X, LEFT_Y, RIGHT_X, RIGHT_Y. Names ignore case, and a name in none
                     of those tables is an error that lists them. STATE is `down`, `up` or `tap` for
                     a button, or a number — or `center` — for an axis, which the pad clamps into its
                     own published range. `tap` is TWO turns, down now and released by the next pump,
                     for the same reason --hold exists: a pack that polls the buttons currently down
                     cannot see a press that opens and closes inside one dispatch, and says so by
                     simply not moving. A --pad named before the pack has created `AEECLSID_HID` is
                     not an error and does nothing, because there is no device there yet to hear it —
                     so when a run looks like it ignored this flag, read --coverage's classes for
                     whether the pack ever asked for the pad at all.
  --wait MS          let MS of virtual time pass before the input after it, so a key can be
                     pressed in a state the app reaches seconds in (repeatable)
  --bmp PATH         write the framebuffer to PATH as a 24-bit BMP
  --dump-audio DIR   write every audio source the run named into DIR, one file per clip, and list
                     them. No clip is decoded into samples here, so this is what a run can show for `ISoundPlayer`:
                     a game names its clips rather than handing them over, and this follows the name
                     — a file for `SDT_FILE`, guest memory for `SDT_BUFFER` — out to the bytes.
  --dump-wav DIR     render the clips this host can play into DIR as 16-bit stereo WAV, through the
                     same mixer the sound card is fed, and write nothing for the ones it cannot play.
                     `--dump-audio` gives you the pack's bytes; this gives you something to listen to on
                     a machine with no sound card, which is the only way a headless run's audio can be
                     checked by a person at all. Clips longer than 60 s are cut and said to be cut.
  --font FILE.bbf    draw with that bitmap font (made by `--make-bbf`) instead of the built-in face.
                     Installed as `AEE_FONT_NORMAL`; bold and large keep the built-in face unless
                     `--font-bold`/`--font-large` name their own files. A run without this flag is
                     unchanged. BBF version 2 is what `--make-bbf` emits, because a CJK-sized
                     repertoire does not fit the older one-row layout.
  --font-bold FILE.bbf
  --font-large FILE.bbf
                     the same for `AEE_FONT_BOLD` / `AEE_FONT_LARGE`. Each file is its own face at
                     its own size, so a pack that asks for the large system font gets the size the
                     file was rasterized at, not a magnification of the normal one.
  --lib PATH         load a library `.mod` beside the pack's own image. A handset's system
                     libraries (a 3D runtime, a billing module) are preloaded before an applet
                     starts, and a pack that calls CreateInstance for one of their classes gets
                     that object. Repeatable; loaded in the order given, each at its own base.
  --soundfont PATH   play the run's clips through that SoundFont (.sf2) while `--window` is up,
                     instead of the tone tables this host carries. The usual General MIDI banks are
                     GPL and this crate is not, so none is vendored: what a run without this flag
                     hears is the built-in bank — eight waveform tables generated at start-up and a
                     mapping of the 128 program numbers onto them that is ours, not a sound
                     register's. A *headless* run still has no sound, because there is no device
                     there to hear it, and the run says which bank it opened.
  --audio-backend NAME
                     decoder backend for compressed audio: `rust` (default; QCP + Symphonia) or
                     `ffmpeg` (reserved optional backend, not built into this binary; selecting it
                     is an explicit error rather than a silent fallback to rust).
  --record-input PATH
                     write the keys and pointer contacts made in a windowed run to PATH. Each line
                     is tab-separated: virtual ms, wall ms, event, key, held ms, detail. The manager
                     uses the same recorder when `QUOIN_INPUT_LOG` names a path before it starts.
  --input-log PATH
                     replay a TSV written by --record-input. Key down/up edges keep their logged
                     virtual times, and the run waits five seconds after the last event. Pair it
                     with `--start auto`; pointer records are ignored by this replay.
  --jit / --no-jit  arm / disarm the optional Rynarmic execution backend. A default build
                     arms it; `--no-jit` keeps the interpreter.
                     A build without that feature has no backend, so asking for `--jit`
                     is an explicit error rather than a silently slower run. Diagnostic runs
                     (--trace, --sample, --break, --watch) still step one instruction at a time.
  --jit-budget N     how many new basic blocks the backend may compile in one turn (default 512,
                     about 2 ms of work). The guest reaches new code constantly — a level load or
                     a scene change can want thousands of blocks at once — and compiling them all
                     inside one turn freezes the picture for as long as it takes. Over the cap the
                     blocks run interpreted for the rest of that turn and are compiled by the turns
                     after it, so a scene change costs many slightly slow frames instead of one long
                     stall. `0` removes the cap (compile as much as the guest asks for), which is
                     how the backend behaved before this flag. Needs a build with `--features jit`.
  --jit-arena N      how many compiled blocks the backend may hold before it throws the whole cache
                     away and starts again (default 65536). Too small is the expensive mistake: a
                     ceiling below the pack's own working set is met again on every pass, which is
                     what made the old 16384 flush for 3.2 s at 17000 blocks. Too large only spends
                     code arena (the 32 MiB of machine memory is reserved once and committed as
                     blocks are written), so the default is generous on purpose. `0` removes the
                     ceiling and leaves that arena as the only limit. `auto` sizes it from the pack
                     itself instead: one block per 4 bytes of module image — an ARM instruction's
                     width, so no ARM pack can want more than this hands it — floored at 4096 and
                     never above 65536. The packs in this tree run from 4.5 KiB to 351 KiB, so the
                     default is sized for the widest one and hands the narrowest 58 times the room
                     its own code could need. It is a bound, not a reading of what a pack uses, which
                     is why it is off unless asked for. Needs a build with `--features jit`; the game
                     library reads the same choice from QUOIN_JIT_ARENA (`auto` or a count) because it
                     has no command line.
  --window [egui|raw]
  --fps / --no-fps  show or hide the guest presentation-rate overlay (default: show).
                     show the screen in a window and play it from the keyboard: the arrows move,
                     Enter is SELECT, Z and X are the soft keys, Backspace is CLR and Esc quits.
                     `egui` is the default and draws the handset's keys as buttons as well; `raw` is
                     the dependency-free window for a machine with no usable GPU. Both follow the
                     wall clock, so these are the runs that are not reproducible to the byte — the
                     scripted forms above still are.
  --speed R          run the game's clock at R times real time: 0.25 to 4. Above 1 the worker opens
                     a turn more often (2x = every 8 ms) and still advances one 16 ms frame; below 1
                     it keeps one turn per frame and advances less (0.5x = 8 ms). Audio is resampled
                     with the same factor, so a faster game is also a higher-pitched one. The actual
                     rate is capped by how fast this host can run the guest; QUOIN_STATS shows the
                     measured rate beside the requested one. Off by default: without it every turn is
                     a fixed 16 ms and a scripted run replays byte for byte; with it the run depends
                     on host speed and does not. Window runs only — a headless --serve run is driven
                     by POST /advance and has no wall-clock turn to align. For the game library (no
                     arguments), set QUOIN_SPEED=2000 (or QUOIN_ALIGN_CLOCK=1 for 1x).
  --align-clock      the 1x spelling of --speed: follow the wall clock and pay a stall back, at most
                     8 extra virtual ms per turn. Kept for scripts that already use it.
  --skin NAME        the handset the egui window draws around the screen: graphite (default),
                     silver or candy. The keys are placed from the device's own screen size, so a
                     176x220 pack and a 320x240 one get different bodies, not different layouts.
  --lang TAG         the language this host's own words come in: a date control's weekday and month
                     names, and the order a whole date is written in. en (default), zh, ja, ko, fr,
                     de. A tag with a region is answered by its language (en-GB is English); a tag
                     this build does not carry is an error rather than a silent fallback.
  --serve [ADDR]     listen for the control interface, so a script can drive a run that is already
                     going: post keys and events, ask for the screen, read what the pack has said.
                     ADDR is a port (`8765`) or host:port (default 127.0.0.1 on any free port, which
                     is printed at the start), and `GET /` lists the verbs. With a window the guest
                     still follows the wall clock; without one it moves only when asked
                     (`POST /advance`), so a session driven this way stays reproducible.
  --net-proxy HOST:PORT
                     send the pack's own network requests to that local proxy for real, and hand what
                     comes back to the guest through `IWebResp`. A developer flag: it is how the
                     requests a pack makes are watched, and answered by a proxy of your own. Without
                     it a run answers every network question the way a handset with no data
                     connection does, and stays reproducible to the byte; with it the answers come
                     from the wall clock. The requests and the answers both show up as the `net` kind
                     of the log `--serve` serves, in the order they happened.
  --net-script FILE  answer the pack's requests from a local route table instead of a socket. Each
                     route is one tab-separated line:
                     `METHOD<TAB>/path<TAB>status<TAB>content-type<TAB>body-file`; `*` matches any
                     method, a trailing `*` on the path is a prefix, and `-` means an empty body.
                     Unmatched requests get a normal 404. A line whose method is `SOCKET` answers a
                     raw socket instead: `SOCKET<TAB>host:port<TAB>reply-file` (host is an IPv4
                     literal or `*`), so a pack that speaks its own protocol gets its bytes too.
                     This is the offline-server flag: the origin no longer has to exist.
                     Mutually exclusive with --net-proxy.
  --scripts DIR      root of per-application Lua 5.4 packages. The package selected for the started
                     applet is DIR/<app-class-in-hex>/manifest.lua; a missing package is not an
                     error. Read-only by default: io/os/package/debug/dofile/loadfile/require are
                     removed, memory/register writes are refused, and each hook has an instruction
                     budget.
  --script-writes    opt the Lua package into destructive guest access: mem.write*/reg.set. Use only
                     for trusted local scripts; without this flag they are safe-mode readers.
  --info             ask the pack what it is, without running it: its module header and entry state,
                     what the loader would say about it, the app's own `AEEAppInfo` record, every
                     container mounted beside it with a kind/MIME census and what could not be decoded,
                     its language strings and audio, the files a run cannot reach, and the screen size
                     argued for by the pack's own art. Pointed at a directory it reports every pack in
                     it and ends with where the art and the directory name disagree. Builds no runtime
                     and retires no instruction, so this is safe to run over a whole corpus — which is
                     the point: `--device` answered wrongly is silent, and the only cheap witness is
                     the picture the pack was drawn for.
  --save-dir DIR     where the app's own writes go. Everything the run creates or edits is mirrored
                     into DIR as it happens, and what an earlier run left there is loaded first, so a
                     game remembers its save between runs. The default is a `saves/<pack>-<hash>`
                     directory beside this executable, one per pack, because a save that vanishes
                     with the run is what 'the game keeps starting over' reads as. `--save-dir none`
                     turns it off, which is what a regression run wants: a run with a home directory
                     is only reproducible from the save it started with. The pack's own directory is
                     never written to, either way.
  --watch ADDR       record every read and store of the four bytes at ADDR, saying which way it went
                     and who did it, and print the record at the end of the run. Whoever did it is
                     said twice: the guest instruction, and — when the host was the one writing — the
                     method and every argument word it was handed. Below the accesses, the same span
                     gets its **arena** history: every `alloc` that handed this space out (naming the
                     instruction the block was given back at, when an earlier free names it), every `free`
                     that put it back, and every `free-refused` where `FREE(p)` named space this arena
                     never owned — a second release of one block, which is what a stale pointer looks
                     like from the outside. Each carries the host call responsible. `alloc` and `free`
                     move no bytes, so without that half a watchpoint cannot tell a live block from
                     one somebody already returned. ADDR takes a number
                     or a region offset (`heap+0x2c`, `stack-0x10`). When the host made the access it
                     names the method and its argument words too, because the call log that would
                     otherwise say so is a capped window and drops the oldest entries
  --watch ADDR,LEN   record accesses across a span of LEN bytes instead, armed after the load,
                     because the loader's own copy is not in the record
                     Repeatable, and the addresses accumulate rather than replacing one another: the
                     question a watchpoint is asked is usually `which of these did the guest touch`,
                     and one run per address cannot tell a shared cause from a coincidence. The report
                     prints a section per address, including the ones that saw nothing
  --watch-stop       stop the run the first time the watchpoint is reached, instead of only recording
                     it, and print the registers and the instruction there. What someone watches a byte
                     for is the state at the access, and by the end of a run those registers hold
                     whatever came last
  --watch-chain      with `--watch`, also walk the heap's free chain at every recorded arena handover
                     and put the first disagreement on that line. It is the reading that dates heap
                     damage — the event whose chain already disagrees is the one just after the write
                     — but one walk covers the whole arena and a pack allocates every frame, so it is
                     asked for rather than paid by default
  --break ADDR[,N]   stop the run before the instruction at ADDR, which takes the same forms as
                     --watch's: a number, a region offset, or a method. Repeatable. A breakpoint is a
                     place to stand *before* something happens, which is the difference between it and
                     --watch-stop: one names an instruction, the other names the data it touches.
                     `,N` waits for the N-th arrival at that address — the call that explains a pack is
                     often the third one through a function that runs once per record — and the report
                     says which arrival stopped the run.
                     `,rX=VAL` waits instead for the arrival where a register holds VAL (r0…r12, sp,
                     lr; 0x… or decimal). Counting is the wrong instrument when a function runs
                     thousands of times and one call is the failing one, because every step of that
                     binary search is a whole boot of the pack. The two combine: `,2,r0=0` ignores the
                     first arrival and stops at the next one that holds zero. Either way the report
                     prints the arrival tally, including zero, so a breakpoint that never fired cannot
                     read as a run that stopped where it was told to.
  --sample N         record where the guest was every N retired instructions (at least 64), keeping the
                     last 4096, and print them at the end: the busiest addresses in that window and the
                     last twenty in time order. This is the one that answers where it was stuck while
                     you keep PLAYING — unlike --break it never stops the run, and unlike --trace it
                     prints nothing while it goes. A hang inside one pack function shows up as one
                     address holding most of the window; a pack that still returns to the shell and
                     simply never gets what it waits for shows a spread instead, and then its host
                     calls are the lines to read next.
  --args PATTERN     in the call log, also spell out the argument words of every call whose name
                     contains PATTERN, e.g. --args BitBlt for what a blit was told to copy.
                     Repeatable, and the patterns add up: --args BitBlt --args FillRect answers both
                     questions in one report. Asking for arguments implies the log that carries them,
                     so this alone is enough — --verbose is only needed for the rest of the volume.
  --disasm [ADDR][,N]
                     disassemble N instructions (default 16) at ADDR, which may be a number, a
                     region offset written the way the reports print one (`module+0x174`,
                     `heap+0x2c`, `stack-0x10`) or a method (`IShell::CreateInstance`). Leave ADDR
                     out to list where the run stopped: a fault's program counter, or the stop a
                     `--break` / `--watch-stop` recorded. A run that neither faulted nor stopped on
                     purpose, with no ADDR given, says there is nothing to list rather than going
                     silent. The register dump names the values that have names.
  --dump ADDR[,LEN]
                     print LEN bytes (default 64) at ADDR as hex and text, and — the part a hex dump
                     cannot give you — name every word in them that is a pointer or a stub address.
                     ADDR takes the same forms as --disasm's, and one more: a register name (r0…r12,
                     sp, lr) dumps what that register points at, which is how an object the pack holds
                     gets read without anyone copying a heap address from an earlier run into a shell.
                     A register reads the same machine the report above it prints: the copy taken at a
                     fault, or the one a `--break` / `--watch-stop` left behind, and only when the run
                     left no copy the live registers after the last instruction. Every line a register
                     dump prints — the dump and the refusal to read it — names which of the three.
  --coverage         print what the run asked the host for: every interface method and helper it
                     reached, with a call count and the busiest first, plus the classes and resources
                     nobody served. A pack describes its own work list this way — it reached 7 of 117
                     helpers and stopped at sprintf — where a guess at what it might use cannot.
  --flame            fold the run into a call tree and print it at the end: one row per frame,
                     indented by depth, with wall clock, instruction count, call count and how much
                     of the row was its own work. This is the run's own account of where it spent
                     itself: a host method is wide here even though the guest retires no instructions
                     while it runs, which is the half `--coverage`'s call counts cannot show. It costs
                     an `Instant::now()` per frame, so a run that did not ask for it does not pay it.
                     The window's developer page draws the same tree, from the same records.

A class id, event or key may be given as a number (0x… or decimal) or, when the SDK publishes the
name in a header, by that name: --event EVT_APP_STOP and --key AVK_DOWN are read exactly as
--event 0x1 and --key 0xe031. Events and keys are queued in the order they appear on the command
line, after --start, and delivered by one turn of the dispatch loop — so an applet that answers a
key by arming a timer or posting an event to itself is heard in the same run.

A .mod is entered at offset 0 as int AEEMod_Load(IShell*, void* ph, IModule** ppMod), where the
second argument is the helper table. An SDK build is the linker's ER_RO segment as a flat blob; a
shipped pack carries a BREW-headered image that relocates itself, and both are read the same way.";

/// Which front end shows the run, when a person is watching rather than a script.
#[derive(Clone, Copy)]
enum Gui {
    /// egui: the screen, and a keypad drawn from `keymap::KEYS` beside it.
    Egui,
    /// The raw operating-system window. No dependency and no GPU, which is the reason it stays.
    Raw,
}

/// One scripted interaction, kept in command-line order because a session is a sequence and not a
/// set: a `--key` before an `EVT_APP_START` would be answered by a different applet state.
/// Say once, on the default output, that scripted input has nowhere to go.
///
/// Not a [`Runtime`](quoin::Runtime) note: `Display::note` deduplicates, drains its oldest half at a
/// cap, and prints only behind `--verbose`, so a fact this load-bearing would be missing from exactly
/// the runs that need it — a capture screen that shows a clean exit code and a frame, for a session
/// that received none of the keys. Returns the flag so the caller cannot forget to set it, and so a
/// second applet adopted in the same process gets its own line when its own session ends.
fn note_ignored(printed: bool, idle: bool) -> bool {
    if !printed {
        if idle {
            // `CloseApplet(TRUE)` succeeded: the handset would be running its idle applet now, and
            // there is no guest left for a key to reach. That is a normal end, not a failed start.
            println!("input ignored: the applet returned to the shell's idle applet");
        } else {
            println!("input ignored: no applet is running, so this session is over");
        }
    }
    true
}

enum Cmd {
    Event {
        evt: u32,
        wp: u32,
        dwp: u32,
    },
    /// One pointer contact. The payload is built when the command runs rather than when the
    /// command line was read, because it costs guest memory and carries the virtual clock.
    Pointer {
        evt: u32,
        x: i32,
        y: i32,
    },
    /// A whole touch — down, a moment with it down, up — which is what a player means by a tap.
    Tap {
        x: i32,
        y: i32,
    },
    Key(u32),
    /// A key edge from a recorded log. Unlike [`Cmd::Key`], the press and release are separate
    /// commands: the log carries the real hold time, and approximating it with the global `--hold`
    /// would make the replay a different session.
    KeyDown(u32),
    KeyUp(u32),
    /// One control of the virtual joystick the console reports through `IHID`, already resolved
    /// against the pad's published tables. Like a key, it is *queued*: the change lands in the job
    /// queue and the next turn of the dispatch loop is what lets the pack hear it, so a scripted
    /// press and a real one take the same road and a run reproduces either way.
    Pad(PadInput),
    /// Let the clock run this many milliseconds before the input after it is posted, so a session
    /// can press a key while the app is in a state it reaches seconds later.
    Wait(u32),
}

/// What to start. `Auto` asks the module which of the class ids in its own image it will build an
/// applet for, which is the only way in from a `.mod` that publishes no list of its classes.
#[derive(Debug, PartialEq, Eq)]
enum Start {
    Cls(u32),
    Auto,
}

/// What the debugging flags asked for, kept together because they are answered in the same two places
/// — the report of a run that ended, and the report of one that faulted. Each of those places would
/// otherwise need a parameter per flag, and a signature that grows with every new one is a signature
/// that eventually gets a call site wrong.
#[derive(Default)]
struct Debug {
    disasm: Option<(Option<String>, usize)>,
    /// `--dump ADDR[,LEN]`: the bytes at `ADDR` and, more to the point, what the words in them point
    /// at.
    dump: Option<(String, usize)>,
    /// `--coverage`: what the run asked the host for, folded by interface and by method.
    coverage: bool,
}

fn main() -> ExitCode {
    quoin::platform::console::utf8_console();
    quoin::library::migrate_legacy_data();
    let args: Vec<String> = std::env::args().skip(1).collect();
    // `--speed` (and its `--align-clock` spelling) is an entry-level switch: the session worker
    // reads QUOIN_SPEED once, the same `std::env::set_var` bridge `--record-input` uses, so the
    // window and the game library share one implementation. Parsed here rather than only in `run`,
    // because a bare `quoin --align-clock` reaches the library without going through it.
    let speed = match parse_speed(&args) {
        Ok(speed) => speed,
        Err(why) => {
            eprintln!("error: {why}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(permille) = speed {
        std::env::set_var("QUOIN_SPEED", permille.to_string());
    }
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{}", USAGE);
        return ExitCode::FAILURE;
    }
    // Nothing to run and nothing asked for is a person who double-clicked this: they expect a list of
    // games, not a usage message. `quoin <file.mod>` is untouched, so a script sees no change. The
    // one exception is `--align-clock` on its own: that is a person asking the library for the
    // option, not a broken command line. `--speed R` carries a value, so it does not get this
    // treatment; the library reads QUOIN_SPEED instead.
    // The library is a window, so it goes with the `ui` feature. A build without it still parses
    // every flag the same way and says why this one is missing rather than pretending the command
    // line was wrong.
    #[cfg(not(feature = "ui"))]
    if args.is_empty() || (args.len() == 1 && args[0] == "--align-clock") {
        eprintln!(
            "error: this build has no game library: it was made without the `ui` feature, which is \
             what links the window toolkit. Name a .mod to run, or use --window raw."
        );
        return ExitCode::FAILURE;
    }
    #[cfg(feature = "ui")]
    if args.is_empty() || (args.len() == 1 && args[0] == "--align-clock") {
        // A person who double-clicked this has nothing to put flags on, so the two things the manager can
        // be told are environment variables. Empty counts as unset: on Windows a variable set to nothing
        // is how a person clears one.
        let skin = std::env::var("QUOIN_SKIN").unwrap_or_default();
        let skin = if skin.trim().is_empty() {
            "graphite"
        } else {
            skin.trim()
        };
        let soundfont = std::env::var("QUOIN_SOUNDFONT").ok();
        let soundfont = soundfont
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        return match quoin::manager::run(&library_root(), skin, soundfont) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("error: {message}");
                ExitCode::FAILURE
            }
        };
    }
    match run(&args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {}", message);
            ExitCode::FAILURE
        }
    }
}

/// Where the library looks for packs.
///
/// `BREW_GAME_DIR` first — the same variable the gated integration tests use, so a person who has already
/// pointed this project at a corpus does not have to point it at it twice — and `Game` beside the working
/// directory otherwise, which is how this repository lays one out.
#[cfg(feature = "ui")]
fn library_root() -> std::path::PathBuf {
    match std::env::var_os("BREW_GAME_DIR") {
        Some(dir) if !dir.is_empty() => std::path::PathBuf::from(dir),
        _ => std::path::PathBuf::from("Game"),
    }
}

/// `--make-bbf FONT[.ttf] [--out FILE.bbf] [--size N] [--face-index N] [--half-widths N]`.
///
/// A build-time conversion, not a run: nothing here touches a pack, the CPU or the display. The
/// output is BBF version 2, because versions 0 and 1 cap the repertoire at a `u16` bitmap width and
/// a CJK-sized face does not fit.
fn make_bbf(args: &[String]) -> Result<ExitCode, String> {
    let mut font: Option<&str> = None;
    let mut out: Option<&str> = None;
    let mut options = quoin::makebbf::Options::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--out" => out = Some(it.next().ok_or("--out needs a path")?),
            "--size" => options.size = take_number(&mut it, "--size")?,
            "--face-index" => options.face_index = take_number(&mut it, "--face-index")?,
            "--half-widths" => options.half_widths = take_number(&mut it, "--half-widths")?,
            "--threshold" => options.threshold = take_number(&mut it, "--threshold")?,
            "--version" => options.version = take_number(&mut it, "--version")?,
            "--range" => {
                let v = it.next().ok_or("--range needs LO-HI")?;
                let (lo, hi) = v
                    .split_once('-')
                    .ok_or_else(|| format!("--range wants LO-HI, got {v}"))?;
                let lo = parse_u16(lo)?;
                let hi = parse_u16(hi)?;
                if lo > hi {
                    return Err(format!("--range wants LO <= HI, got {v}"));
                }
                options.ranges.push((lo, hi));
            }
            other if other.starts_with("--") => {
                return Err(format!("--make-bbf does not take {other}"))
            }
            other => font = Some(other),
        }
    }
    let font = font.ok_or("--make-bbf needs a font file")?;
    let data = std::fs::read(font).map_err(|e| format!("read {font}: {e}"))?;
    let (bytes, report) = quoin::makebbf::from_ttf(&data, &options)?;
    let out = out.map(str::to_string).unwrap_or_else(|| {
        let mut path = std::path::PathBuf::from(font);
        path.set_extension("bbf");
        path.to_string_lossy().into_owned()
    });
    std::fs::write(&out, &bytes).map_err(|e| format!("write {out}: {e}"))?;
    let cell = quoin::bbf::Bbf::parse(&bytes)
        .map(|face| face.height())
        .unwrap_or(options.size);
    println!(
        "wrote {out}: {} bytes, {} glyphs, {cell}px cell",
        bytes.len(),
        report.glyphs
    );
    if report.outside_bmp > 0 {
        eprintln!(
            "note: {} code point(s) above U+FFFF are not in the output; BBF ranges are u16",
            report.outside_bmp
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn take_number<T: std::str::FromStr>(
    it: &mut std::slice::Iter<'_, String>,
    flag: &str,
) -> Result<T, String> {
    let value = it.next().ok_or_else(|| format!("{flag} needs a value"))?;
    value
        .parse()
        .map_err(|_| format!("{flag} is not a number: {value}"))
}

/// A code point bound, decimal or `0x`-prefixed.
/// `--speed R` and its `--align-clock` spelling, as thousandths of real time.
///
/// Parsed before `run` so the game-library branch (no pack) sees it too, and so a bad value is an
/// error rather than a run that quietly stayed at 1x.
fn parse_speed(args: &[String]) -> Result<Option<u32>, String> {
    let mut speed = None;
    let mut align = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--align-clock" => align = true,
            "--speed" => {
                let value = it
                    .next()
                    .ok_or("--speed needs a value between 0.25 and 4")?;
                speed = Some(parse_speed_value(value)?);
            }
            _ => {}
        }
    }
    match (align, speed) {
        (true, Some(p)) if p != quoin::session::SPEED_ONE => Err(format!(
            "--align-clock is --speed 1, so it cannot be combined with --speed {}",
            p as f32 / 1000.0
        )),
        (true, _) => Ok(Some(quoin::session::SPEED_ONE)),
        (false, speed) => Ok(speed),
    }
}

fn parse_speed_value(text: &str) -> Result<u32, String> {
    let value: f64 = text
        .trim()
        .trim_end_matches('x')
        .parse()
        .map_err(|_| format!("--speed wants a number between 0.25 and 4, got {text}"))?;
    let min = f64::from(quoin::session::SPEED_MIN) / 1000.0;
    let max = f64::from(quoin::session::SPEED_MAX) / 1000.0;
    if !(min..=max).contains(&value) {
        return Err(format!(
            "--speed wants a number between {min} and {max}, got {text}"
        ));
    }
    Ok((value * 1000.0).round() as u32)
}

fn parse_u16(text: &str) -> Result<u16, String> {
    let text = text.trim();
    let parsed = if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u16::from_str_radix(hex, 16)
    } else {
        text.parse()
    };
    parsed.map_err(|_| format!("not a number: {text}"))
}

/// Read a BBF and install it under one `AEEFont` designator.
fn install_bbf(rt: &mut Runtime, path: &str, designator: u32) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let face = quoin::bbf::Bbf::parse(&bytes).map_err(|e| format!("{path}: {e}"))?;
    rt.display.install_face(
        designator,
        quoin::font::Face::Bbf(std::sync::Arc::new(face)),
    );
    Ok(())
}

/// Flags that expose diagnostics, tracing or developer-only plumbing. They are accepted only with
/// an explicit `--dev`; ordinary runs stay on the short, stable surface.
const DEVELOPER_FLAGS: &[&str] = &[
    "--trace",
    "--sample",
    "--trace-limit",
    "--trace-range",
    "--trace-events",
    "--disasm",
    "--dump",
    "--break",
    "--watch",
    "--watch-stop",
    "--watch-chain",
    "--coverage",
    "--flame",
    "--args",
    "--verbose",
    "--info",
    "--heap-mb",
    "--heap-initial-kb",
    "--ram-mb",
    "--color-depth",
    "--encoding",
    "--thumb-entry",
    "--bmp",
    "--dump-audio",
    "--dump-wav",
    "--record-input",
    "--input-log",
    "--serve",
    "--net-proxy",
    "--net-script",
    "--scripts",
    "--script-writes",
];

fn developer_flag(args: &[String]) -> Option<&str> {
    args.iter()
        .map(String::as_str)
        .find(|arg| DEVELOPER_FLAGS.contains(arg))
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    // A font conversion runs before any run state exists, so it works on a machine with no pack and
    // no display.
    if let Some(at) = args.iter().position(|a| a == "--make-bbf") {
        return make_bbf(&args[at + 1..]);
    }
    if !args.iter().any(|arg| arg == "--dev") {
        if let Some(flag) = developer_flag(args) {
            return Err(format!(
                "{flag} is a developer option; add --dev (run `quoin --dev ...`)"
            ));
        }
    }
    let mut path: Option<&str> = None;
    let mut trace = Trace::default();
    // `--sample N`: keep one program counter every N retired instructions. 0 is not armed.
    let mut sample: u64 = 0;
    // `--flame`: fold the run's dispatches and downcalls into a call tree, and report it at the end.
    // Off unless asked for, because an armed tree costs an `Instant::now()` per frame (see
    // `quoin::profile`), and a run that did not ask should not pay for the instrument.
    let mut flame = false;
    let mut thumb = false;
    let mut verbose = false;
    let mut steps: Option<u64> = None;
    let mut steps_ceiling: Option<u64> = None;
    let mut session_ceiling: Option<u64> = None;
    let mut start: Option<Start> = None;
    let mut device: Option<(u16, u16)> = None;
    // `--device-json FILE`: the handset read from a file instead of from `DeviceProfile::standard`.
    // Absent means the built-in one, so no run changes shape by not naming a file.
    let mut device_json: Option<String> = None;
    // `--save-device`: write the profile *after* every flag has had its say, which is how a person
    // turns a set of flags they got right into a file they can edit.
    let mut save_device = false;
    // `--heap-mb` / `--heap-initial-kb` / `--ram-mb`: the three memory figures a run is given.
    // `None` keeps each one at the delivery's own value (240 MiB, 256 KiB, 64 MiB), so a run that
    // names none of them is the machine it always was.
    let mut heap_mb: Option<u32> = None;
    let mut heap_initial_kb: Option<u32> = None;
    let mut ram_mb: Option<u32> = None;
    // Outer `Option` is "was the flag given at all", inner is the cap itself (`None` = no cap, i.e.
    // `--jit-budget 0`). Keeping the two apart is what lets an absent flag mean "leave the default"
    // while an explicit 0 means "compile as much as the guest asks for".
    let mut jit_budget: Option<Option<u32>> = None;
    // `--jit-arena`: the compiled-block ceiling. `None` keeps the backend's own 65536 — including
    // for `--jit-arena auto`, which is the whole point of the flag being off unless asked for.
    let mut jit_arena: Option<quoin::cpu::JitArena> = None;
    let mut color_depth: Option<u16> = None;
    let mut encoding: Option<u16> = None;
    let mut bmp: Option<String> = None;
    let mut dump_audio: Option<String> = None;
    let mut dump_wav: Option<String> = None;
    let mut soundfont: Option<String> = None;
    // `--lib PATH`: a library `.mod` (no applet class) the pack expects the handset to have
    // loaded already. Repeatable, and loaded in the order given.
    let mut libraries: Vec<String> = Vec::new();
    // `--font FILE.bbf`: a bitmap face made by `--make-bbf`, installed as `AEE_FONT_NORMAL`.
    let mut font: Option<String> = None;
    let mut font_bold: Option<String> = None;
    let mut font_large: Option<String> = None;
    let mut audio_backend = AudioBackend::Rust;
    let mut record_input: Option<String> = None;
    // `--trace-events FILE`: every applet event, as TSV, flushed per event so a fault still leaves
    // the sequence that led to it.
    let mut trace_events: Option<String> = None;
    let mut gui: Option<Gui> = None;
    let mut show_fps = true;
    // `--jit` / `--no-jit`: arm or disarm the optional Rynarmic backend. `None` means the
    // caller did not choose, so a build with the feature arms it by default. A build without
    // the feature has no backend, and an explicit `--jit` is an error rather than a silently
    // slower run.
    let mut jit: Option<bool> = None;
    let mut skin: Option<String> = None;
    // `--lang TAG`: the language this host's own words come in — a date control's weekday and month
    // names. Deliberately not derived from `AEEDeviceInfo::dwLang`: that family's values are not in
    // this tree's tables, so a mapping from it would be a number nobody could check.
    let mut lang: Option<String> = None;
    // `--serve [ADDR]`: the address is optional, so an empty string means "any free loopback port".
    let mut serve: Option<String> = None;
    // `--save-dir DIR`: where the app's own writes are mirrored, so a game can keep a save.
    let mut save_dir: Option<String> = None;
    // `--net-proxy HOST:PORT`: where the guest's own requests are really sent, if anywhere.
    let mut net_proxy: Option<String> = None;
    // `--net-script FILE`: answer the guest's requests from a local route table instead of a socket.
    let mut net_script: Option<String> = None;
    // `--scripts DIR`: root of per-application Lua packages; `--script-writes` opts into the
    // destructive guest APIs (default is read-only).
    let mut scripts_dir: Option<String> = None;
    let mut script_writes = false;
    // `--info`: describe the pack from its own bytes. A report, not a run — nothing here builds a
    // Runtime, so it is safe over a whole directory of games.
    let mut info = false;
    let mut log_args: Vec<String> = Vec::new();
    let mut watch: Vec<(u32, Option<u32>)> = Vec::new();
    // `--watch-stop`: whether reaching the watchpoint ends the run instead of only being recorded. The
    // registers at the access are the reason someone watches a byte at all, and by the end of a run they
    // are the registers of whatever came last.
    let mut watch_stop = false;
    // `--watch-chain`: whether each recorded arena handover also walks the free chain. It is the only
    // reading that dates the damage, and the only one whose cost is the whole arena.
    let mut watch_chain = false;
    let mut break_at: Vec<String> = Vec::new();
    // `--disasm` and `--dump`: the two flags that answer "what was it doing" and "what is in there",
    // and the two that are answered in the same places, so they travel as one.
    let mut debug = Debug::default();
    let mut run_for: u32 = 0;
    // How long a scripted `--key` holds the key down, in virtual milliseconds. A human tap is
    // tens of them; this is one tick of the clocks the shipped packs arm.
    let mut hold: u32 = 150;
    let mut cmds: Vec<Cmd> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let take = |it: &mut std::slice::Iter<'_, String>, flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match a.as_str() {
            "--dev" => {}
            // `main` installed these as QUOIN_SPEED before parsing; the worker reads it there.
            "--align-clock" => {}
            "--speed" => {
                let _ = take(&mut it, "--speed")?;
            }
            "--trace" => trace.limit = u64::MAX,
            "--sample" => {
                // Instructions between two samples. The floor is there because a person typing a small
                // number means "fine-grained", and at 1 this is a full trace kept in memory: 4096
                // entries would cover 4096 instructions, which is not a hang's tail but its last page.
                let v = take(&mut it, "--sample")?;
                let n: u64 = v.parse().map_err(|_| format!("not a number: {v}"))?;
                if n < 64 {
                    return Err(format!("--sample wants at least 64 instructions, got {n}"));
                }
                sample = n;
            }
            "--trace-limit" => {
                let v = take(&mut it, "--trace-limit")?;
                trace.limit = v.parse().map_err(|_| format!("not a number: {v}"))?;
            }
            "--trace-range" => {
                // `LO,HI`, each written the way any other address on this command line is. `module+…`
                // cannot be one of them: a trace is armed before the image is in place, so there is no
                // module for a bound to be relative to yet.
                let v = take(&mut it, "--trace-range")?;
                let (lo, hi) = v
                    .split_once(',')
                    .ok_or_else(|| format!("--trace-range wants LO,HI, got {v}"))?;
                let lo = symbol::parse(None, lo).ok_or_else(|| format!("not an address: {lo}"))?;
                let hi = symbol::parse(None, hi).ok_or_else(|| format!("not an address: {hi}"))?;
                trace.range = Some((lo, hi));
                // A range with no limit is what someone typing one range means, the same way that
                // `--trace-limit` on its own is a request to trace.
                if trace.limit == 0 {
                    trace.limit = u64::MAX;
                }
            }
            "--disasm" => {
                let v = take(&mut it, "--disasm")?;
                let (addr, n) = match v.split_once(',') {
                    Some((a, n)) => (a.trim(), n.trim().parse().unwrap_or(16)),
                    None => (v.trim(), 16),
                };
                debug.disasm = Some((
                    if addr.is_empty() {
                        None
                    } else {
                        Some(addr.to_string())
                    },
                    n.max(1),
                ));
            }
            "--dump" => {
                let v = take(&mut it, "--dump")?;
                let (addr, len) = match v.split_once(',') {
                    Some((a, l)) => (a.trim(), l.trim().parse().unwrap_or(64)),
                    None => (v.trim(), 64),
                };
                if addr.is_empty() {
                    return Err("--dump needs an address".to_string());
                }
                debug.dump = Some((addr.to_string(), len.max(1).min(4096)));
            }
            "--coverage" => debug.coverage = true,
            "--flame" => flame = true,
            "--watch-stop" => watch_stop = true,
            "--watch-chain" => watch_chain = true,
            "--break" => break_at.push(take(&mut it, "--break")?),
            "--thumb-entry" => thumb = true,
            "--verbose" => verbose = true,
            "--steps" => {
                let v = take(&mut it, "--steps")?;
                steps = Some(v.parse().map_err(|_| format!("not a number: {v}"))?);
            }
            "--steps-ceiling" => {
                let v = take(&mut it, "--steps-ceiling")?;
                steps_ceiling = Some(v.parse().map_err(|_| format!("not a number: {v}"))?);
            }
            "--session-ceiling" => {
                let v = take(&mut it, "--session-ceiling")?;
                session_ceiling = Some(v.parse().map_err(|_| format!("not a number: {v}"))?);
            }
            "--start" => {
                let arg = take(&mut it, "--start")?;
                start = Some(if arg.eq_ignore_ascii_case("auto") {
                    Start::Auto
                } else {
                    Start::Cls(resolve(&arg)?)
                });
            }
            "--device" => {
                let v = take(&mut it, "--device")?;
                let Some((w, h)) = v.split_once('x') else {
                    return Err(format!("--device wants WxH, got {v}"));
                };
                let (w, h) = (w.trim().parse::<u16>(), h.trim().parse::<u16>());
                device = Some(match (w, h) {
                    (Ok(w), Ok(h)) if w > 0 && h > 0 => (w, h),
                    _ => return Err(format!("not a screen size: {v}")),
                });
            }
            "--device-json" => device_json = Some(take(&mut it, "--device-json")?),
            "--save-device" => save_device = true,
            "--heap-mb" => {
                let v = take(&mut it, "--heap-mb")?;
                let n: u32 = v.parse().map_err(|_| format!("not a size in MiB: {v}"))?;
                // The arena's own room runs to the stack's floor; a larger answer would overlap it.
                if !(1..=240).contains(&n) {
                    return Err(format!("--heap-mb wants 1..=240, got {n}"));
                }
                heap_mb = Some(n);
            }
            "--jit-budget" => {
                let v = take(&mut it, "--jit-budget")?;
                let n: u32 = v.parse().map_err(|_| format!("not a block count: {v}"))?;
                // 0 is the documented "no cap"; anything larger is a count of blocks per turn.
                if n > 100_000 {
                    return Err(format!("--jit-budget wants 0..=100000, got {n}"));
                }
                jit_budget = Some(if n == 0 { None } else { Some(n) });
            }
            "--jit-arena" => {
                let v = take(&mut it, "--jit-arena")?;
                jit_arena =
                    Some(quoin::cpu::JitArena::parse(&v).map_err(|e| format!("--jit-arena: {e}"))?);
            }
            "--heap-initial-kb" => {
                let v = take(&mut it, "--heap-initial-kb")?;
                let n: u32 = v.parse().map_err(|_| format!("not a size in KiB: {v}"))?;
                if n == 0 {
                    return Err("--heap-initial-kb wants at least 1".to_string());
                }
                heap_initial_kb = Some(n);
            }
            "--ram-mb" => {
                let v = take(&mut it, "--ram-mb")?;
                let n: u32 = v.parse().map_err(|_| format!("not a size in MiB: {v}"))?;
                if !(1..=1024).contains(&n) {
                    return Err(format!("--ram-mb wants 1..=1024, got {n}"));
                }
                ram_mb = Some(n);
            }
            "--color-depth" => {
                // A diagnostic, not a device: the screen this host actually hands out stays RGB565, so
                // changing the advertised depth decouples the two answers `AEEDeviceInfo::nColorDepth`
                // and `AEEBitmapInfo::nDepth` normally agree on. That is worth doing to ask a pack which
                // code path it takes, and is not a way to run a 24 bpp device — see `--help`.
                let v = take(&mut it, "--color-depth")?;
                let n: u16 = v.parse().map_err(|_| format!("not a colour depth: {v}"))?;
                if !matches!(n, 1 | 2 | 4 | 8 | 12 | 16 | 18 | 24 | 32) {
                    return Err(format!("--color-depth: not a depth: {n}"));
                }
                color_depth = Some(n);
            }
            "--encoding" => {
                // `AEEDeviceInfo::wEncoding`, an `AEE_ENC_*` number. This host advertises it and never
                // converts by it, so overriding the number asks the pack what *it* does with that
                // answer — the cleanest device A/B in this struct, because nothing on our side moves.
                // Written the way the header writes them, so `0xFD` is a value and not a typo.
                let v = take(&mut it, "--encoding")?;
                let n = symbol::number(&v)
                    .filter(|n| *n <= u16::MAX as u32)
                    .ok_or_else(|| format!("not an AEE_ENC_* number: {v}"))?;
                encoding = Some(n as u16);
            }
            "--key" => cmds.push(Cmd::Key(resolve(&take(&mut it, "--key")?)?)),
            "--wait" => {
                let v = take(&mut it, "--wait")?;
                cmds.push(Cmd::Wait(
                    v.parse().map_err(|_| format!("not a number: {v}"))?,
                ));
            }
            "--event" => cmds.push(parse_event(&take(&mut it, "--event")?)?),
            "--pointer" => cmds.push(parse_pointer(&take(&mut it, "--pointer")?)?),
            "--pad" => {
                // Resolved while the command line is still the thing being read, because the target
                // names a control off a published table and a name that is not on it should be said
                // before a run has spent twenty seconds getting to the point where it would matter.
                let v = take(&mut it, "--pad")?;
                cmds.push(Cmd::Pad(PadInput::parse(&v)?));
            }
            "--run-for" => {
                let v = take(&mut it, "--run-for")?;
                run_for = v.parse().map_err(|_| format!("not a number: {v}"))?;
            }
            "--hold" => {
                let v = take(&mut it, "--hold")?;
                hold = v.parse().map_err(|_| format!("not a number: {v}"))?;
            }
            "--bmp" => bmp = Some(take(&mut it, "--bmp")?),
            "--dump-audio" => dump_audio = Some(take(&mut it, "--dump-audio")?),
            "--dump-wav" => dump_wav = Some(take(&mut it, "--dump-wav")?),
            "--soundfont" => soundfont = Some(take(&mut it, "--soundfont")?),
            "--lib" => libraries.push(take(&mut it, "--lib")?),
            "--font" => font = Some(take(&mut it, "--font")?),
            "--font-bold" => font_bold = Some(take(&mut it, "--font-bold")?),
            "--font-large" => font_large = Some(take(&mut it, "--font-large")?),
            "--audio-backend" => {
                let v = take(&mut it, "--audio-backend")?;
                audio_backend = AudioBackend::parse(&v)
                    .ok_or_else(|| format!("audio backend is rust or ffmpeg, not {v}"))?;
                if !audio_backend.available() {
                    return Err(format!(
                        "audio backend {} is not built into this binary (available: rust)",
                        audio_backend.name()
                    ));
                }
            }
            "--jit" => jit = Some(true),
            "--no-jit" => jit = Some(false),
            "--fps" => show_fps = true,
            "--no-fps" => show_fps = false,
            "--window" => {
                // The back end is optional, so a following flag belongs to the next option rather
                // than to this one. A word that names neither is a typo, and saying so beats
                // quietly reading it as something else.
                gui = Some(match it.clone().next().map(String::as_str) {
                    None => Gui::Egui,
                    Some("egui") => {
                        it.next();
                        Gui::Egui
                    }
                    Some("raw") => {
                        it.next();
                        Gui::Raw
                    }
                    Some(next) if next.starts_with('-') => Gui::Egui,
                    Some(next) => return Err(format!("--window is egui or raw, not {next}")),
                });
            }
            "--skin" => {
                let v = take(&mut it, "--skin")?;
                // Checked here rather than at the window, because a name that means nothing should be
                // said while the command line is still the thing being read.
                if quoin::handset::skin(&v).is_none() {
                    return Err(format!(
                        "no skin {v}; this build has {}",
                        quoin::handset::skin_names()
                    ));
                }
                skin = Some(v);
            }
            "--lang" => {
                let v = take(&mut it, "--lang")?;
                // Checked while the command line is still the thing being read, the same as `--skin`.
                // An unknown tag would be answered in English rather than failing, so a typo would
                // look exactly like a deliberate fallback; the error is what tells them apart.
                let head = v.split(['-', '_']).next().unwrap_or("").to_string();
                if !quoin::locale::tags()
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(&head))
                {
                    return Err(format!(
                        "no language {v}; this build has {}",
                        quoin::locale::tags().join(", ")
                    ));
                }
                lang = Some(v);
            }
            "--serve" => {
                // The address is optional, so a following flag belongs to the next option:
                // `--serve --window` means "any free port, in a window", and a bare word that is not
                // an address is a typo worth saying rather than reading as one.
                serve = Some(match it.clone().next().map(String::as_str) {
                    Some(next) if !next.starts_with('-') => take(&mut it, "--serve")?,
                    _ => String::new(),
                });
            }
            "--record-input" => record_input = Some(take(&mut it, "--record-input")?),
            "--trace-events" => trace_events = Some(take(&mut it, "--trace-events")?),
            "--input-log" => {
                let path = take(&mut it, "--input-log")?;
                cmds.extend(input_log_commands(&path)?);
            }
            "--save-dir" => save_dir = Some(take(&mut it, "--save-dir")?),
            "--net-proxy" => net_proxy = Some(take(&mut it, "--net-proxy")?),
            "--net-script" => net_script = Some(take(&mut it, "--net-script")?),
            "--scripts" => scripts_dir = Some(take(&mut it, "--scripts")?),
            "--script-writes" => script_writes = true,
            "--info" => info = true,
            // Repeatable, and the patterns accumulate: two questions asked at once are two patterns,
            // and a flag that silently replaced the first with the second would answer neither.
            "--args" => log_args.push(take(&mut it, "--args")?),
            "--watch" => {
                let v = take(&mut it, "--watch")?;
                let (addr, len) = match v.split_once(',') {
                    Some((a, l)) => (a, Some(l)),
                    None => (v.as_str(), None),
                };
                // No module yet, so the span is relative to the map rather than to an image.
                let start = symbol::parse(None, addr).ok_or_else(|| {
                    format!("not an address: {addr} — try 0x…, heap+0x… or stack-0x…")
                })?;
                let len = match len {
                    Some(l) => {
                        Some(symbol::parse(None, l).ok_or_else(|| format!("not a number: {l}"))?)
                    }
                    None => None,
                };
                watch.push((start, len));
            }
            other if path.is_none() && !other.starts_with("--") => path = Some(other),
            other => return Err(format!("unexpected argument: {}", other)),
        }
    }
    if let Some(path) = record_input.as_deref() {
        // `Shared` is built after this parser, and both the window and the manager reach it through
        // the same environment variable. Setting it here keeps one recorder implementation for both.
        std::env::set_var("QUOIN_INPUT_LOG", path);
    }
    // `--save-device` has nothing to write without a file to write it to, and this is where that is
    // decided: below sits `fs::read(path)`, which reads the whole pack before the profile is built,
    // so a check placed at the writing site would make "you forgot --device-json" cost a pack load.
    let save_path = match (save_device, device_json.as_deref()) {
        (true, Some(file)) => Some(file),
        (true, None) => {
            return Err(
                "--save-device writes the file --device-json named, so it needs one to write"
                    .into(),
            )
        }
        (false, _) => None,
    };
    let path = path.ok_or("no .mod given")?;
    if args.iter().any(|a| a == "--align-clock" || a == "--speed") && gui.is_none() && !info {
        // These shape the windowed worker's turn length; a headless run has no such turn. Accepting
        // them and then doing nothing is the "taken, then silent" shape this CLI refuses. `--info`
        // is exempt: it has its own "flags with no target" report and lists them there.
        return Err(
            "--speed needs --window: a headless run is driven by POST /advance, so there is no wall-clock turn to align"
                .into(),
        );
    }
    if info {
        // `--info` starts no guest, so every flag that shapes a run would be taken and then do nothing —
        // and "accepted, then silent" is the bug shape this repository hates most. Name them instead.
        let shaped: Vec<&str> = [
            ("--window", gui.is_some()),
            (
                "--speed/--align-clock",
                args.iter().any(|a| a == "--speed" || a == "--align-clock"),
            ),
            ("--skin", skin.is_some()),
            ("--lang", lang.is_some()),
            ("--device", device.is_some()),
            ("--device-json", device_json.is_some()),
            ("--save-device", save_device),
            ("--heap-mb", heap_mb.is_some()),
            ("--jit-budget", jit_budget.is_some()),
            ("--jit-arena", jit_arena.is_some()),
            ("--heap-initial-kb", heap_initial_kb.is_some()),
            ("--ram-mb", ram_mb.is_some()),
            ("--color-depth", color_depth.is_some()),
            ("--encoding", encoding.is_some()),
            ("--start", start.is_some()),
            ("--key/--event/--pointer/--pad", !cmds.is_empty()),
            ("--run-for", run_for > 0),
            ("--hold", hold != 150),
            ("--bmp", bmp.is_some()),
            ("--dump-audio", dump_audio.is_some()),
            ("--dump-wav", dump_wav.is_some()),
            ("--soundfont", soundfont.is_some()),
            ("--audio-backend", audio_backend != AudioBackend::Rust),
            ("--record-input", record_input.is_some()),
            ("--trace-events", trace_events.is_some()),
            (
                "--input-log",
                cmds.iter()
                    .any(|c| matches!(c, Cmd::KeyDown(_) | Cmd::KeyUp(_))),
            ),
            ("--serve", serve.is_some()),
            ("--jit/--no-jit", jit.is_some()),
            ("--save-dir", save_dir.is_some()),
            ("--net-proxy", net_proxy.is_some()),
            ("--net-script", net_script.is_some()),
            ("--scripts", scripts_dir.is_some()),
            ("--script-writes", script_writes),
            ("--args", !log_args.is_empty()),
            ("--watch", !watch.is_empty()),
            ("--watch-stop", watch_stop),
            ("--watch-chain", watch_chain),
            ("--break", !break_at.is_empty()),
            ("--trace", trace.limit > 0),
            ("--sample", sample != 0),
            (
                "--disasm/--dump/--coverage",
                debug.coverage || debug.disasm.is_some() || debug.dump.is_some(),
            ),
            ("--flame", flame),
            (
                "--steps/--steps-ceiling/--session-ceiling",
                steps.is_some() || steps_ceiling.is_some() || session_ceiling.is_some(),
            ),
            ("--thumb-entry", thumb),
        ]
        .into_iter()
        .filter(|(_, set)| *set)
        .map(|(flag, _)| flag)
        .collect();
        if !shaped.is_empty() {
            return Err(format!(
                "--info runs no pack, so these flags have no target: {}. Either drop them or drop --info",
                shaped.join("  ")
            ));
        }
        // A probe opens files and reads them. It builds no `Runtime` and executes nothing, so this
        // branch is what makes the flag safe to point at a whole directory of games — and it ends the
        // run here, before any of the load-time decisions below could need a guest.
        let target = std::path::Path::new(path);
        let mods = if target.is_dir() {
            quoin::probe::packs(target)
        } else if target.is_file() {
            vec![target.to_path_buf()]
        } else {
            return Err(format!("no such file or directory: {path}"));
        };
        if mods.is_empty() {
            return Err(format!("{path}: no .mod in this directory"));
        }
        let many = mods.len() > 1;
        let reports: Vec<quoin::probe::Probe> =
            mods.iter().map(|m| quoin::probe::Probe::read(m)).collect();
        for p in &reports {
            let label = p.path.display().to_string();
            p.print(many.then_some(label.as_str()));
            println!();
        }
        if many {
            report_probe(&reports);
        }
        return Ok(ExitCode::SUCCESS);
    }
    if skin.is_some() && !matches!(gui, Some(Gui::Egui)) {
        // The skin is the handset's body, and only the egui front end draws one. Saying so beats a
        // flag that appears to have been accepted and then did nothing.
        return Err(
            "--skin needs --window; the raw window draws the screen and nothing around it".into(),
        );
    }
    let image = std::fs::read(path).map_err(|e| format!("{}: {}", path, e))?;

    // A pack is compiled for one screen and sizes its own buffers from what the handset reports, so
    // the profile has to be chosen before the runtime, and with it the display and its bitmap.
    //
    // `--device-json` replaces the built-in handset and the four flags below still override it: a file
    // describes a device, a flag asks "and what if this one thing were different". Same relationship
    // `DeviceProfile::standard` had with them before it could be loaded from anywhere.
    let profile = match device_json.as_deref() {
        Some(path) => {
            let loaded = quoin::device_file::load(std::path::Path::new(path))?;
            if loaded.created {
                println!(
                    "device: {path} did not exist; wrote the built-in handset there, so a run can be \
                     pointed at a file that says what this one does"
                );
            }
            loaded.profile
        }
        None => DeviceProfile::standard(),
    };
    let profile = match device {
        Some((w, h)) => profile.with_screen(w, h),
        None => profile,
    };
    // The two diagnostics override *answers*, not the machine: the frame stays RGB565 and the text
    // helpers stay the ones this host implements. Applied after the screen so a run that gave both gets
    // both, and reported below so a reader of the log cannot mistake this for a different handset.
    let profile = match (color_depth, encoding) {
        (None, None) => profile,
        (d, e) => profile.with_advertised(d, e),
    };
    // `--ram-mb`: the total a pack is told the handset has. It is a different question from the
    // heap's span, which is why the two are separate flags.
    let profile = match ram_mb {
        Some(mb) => quoin::device::DeviceProfile {
            ram: mb * 1024 * 1024,
            ..profile
        },
        None => profile,
    };
    // `--save-device`: the profile as every flag left it. Written here, before the machine is built,
    // so a run that then faults still leaves the file it was asked for — the file is the answer to
    // "what did I just describe", and a fault is exactly when a person wants to look at it.
    if let Some(file) = save_path {
        quoin::device_file::save(std::path::Path::new(file), &profile)?;
        println!("device: wrote {file}");
    }
    let heap_bytes = heap_mb
        .map(|mb| mb * 1024 * 1024)
        .unwrap_or(quoin::layout::HEAP_LIMIT - quoin::layout::HEAP_BASE);
    let heap_initial = heap_initial_kb
        .map(|kb| kb * 1024)
        .unwrap_or(quoin::heap::INITIAL);
    if heap_initial > heap_bytes {
        return Err(format!(
            "--heap-initial-kb {} KiB does not fit the {} KiB span `--heap-mb` gives",
            heap_initial / 1024,
            heap_bytes / 1024
        ));
    }
    let mut rt = Runtime::with_device_and_heap(profile, heap_bytes, heap_initial);
    #[cfg(feature = "jit")]
    if jit.unwrap_or(true) {
        rt.cpu.enable_jit();
        println!("jit: Rynarmic backend armed");
    }
    #[cfg(not(feature = "jit"))]
    if jit == Some(true) {
        return Err("--jit needs a build with --features jit".into());
    }
    // The per-turn compile cap only exists where there is a backend to spend it. Saying so beats
    // taking the flag and quietly ignoring it, which is the shape this CLI refuses elsewhere.
    if let Some(cap) = jit_budget {
        #[cfg(feature = "jit")]
        {
            rt.jit_compile_budget = cap;
        }
        #[cfg(not(feature = "jit"))]
        {
            let _ = cap;
            return Err("--jit-budget needs a build with --features jit".into());
        }
    }
    // The compiled-block ceiling is only meaningful where a cache exists: a build with no backend,
    // or a run told to leave it off, would take the flag and do nothing with it. Both are refused
    // here, before the pack loads. The value itself is applied further down, once the modules are
    // in — `auto` reads their sizes.
    if jit_arena.is_some() {
        #[cfg(not(feature = "jit"))]
        return Err("--jit-arena needs a build with --features jit".into());
        #[cfg(feature = "jit")]
        if jit == Some(false) {
            return Err(
                "--jit-arena sizes the JIT's block cache, and --no-jit leaves no cache to size"
                    .into(),
            );
        }
    }
    let mut explicit_faces = Vec::new();
    if let Some(path) = font.as_deref() {
        // A font is host state, not guest state: the file is read and decoded before the module
        // loads, so a pack that draws during `AEEMod_Load` already sees it.
        install_bbf(&mut rt, path, quoin::abi::generated::AEE_FONT_NORMAL)?;
        explicit_faces.push(quoin::abi::generated::AEE_FONT_NORMAL);
    }
    if let Some(path) = font_bold.as_deref() {
        install_bbf(&mut rt, path, quoin::abi::generated::AEE_FONT_BOLD)?;
        explicit_faces.push(quoin::abi::generated::AEE_FONT_BOLD);
    }
    if let Some(path) = font_large.as_deref() {
        install_bbf(&mut rt, path, quoin::abi::generated::AEE_FONT_LARGE)?;
        explicit_faces.push(quoin::abi::generated::AEE_FONT_LARGE);
    }
    // Whatever the flags left alone gets the machine's own wide face: a Chinese pack's menus are
    // written for an OEM face that covers GB2312, and this host's built-in grid has no ideographs.
    quoin::systemfont::install(&mut rt, &explicit_faces);
    if let Some(path) = scripts_dir {
        rt.script_root = std::path::PathBuf::from(path);
    }
    rt.script_writes = script_writes;
    rt.script_consent_ui = gui.is_some();
    if let Some(path) = trace_events.as_deref() {
        rt.open_event_trace(std::path::Path::new(path))
            .map_err(|e| format!("--trace-events {path}: {e}"))?;
    }
    if let Some(tag) = &lang {
        // Applied before the load, because a control can be created and asked during it. A tag with a
        // region is answered by its language, and saying which one the run got is the difference
        // between a fallback a person meant and one they did not.
        let taken = rt.widgets.set_language(tag);
        if !taken.eq_ignore_ascii_case(tag) {
            eprintln!("--lang {tag}: answering in {taken}");
        }
    }
    rt.cpu.trace = trace;
    if sample != 0 {
        // Armed before the load, like the watchpoints: a pack that dies during `AEEMod_Load` is exactly
        // the run whose tail a reader wants, and a run that never armed one keeps the empty ring.
        rt.cpu.samples = Samples::armed(sample, SAMPLE_CAP);
    }
    rt.log_args = log_args;
    // Armed before the load, like the samples and the watchpoints: a pack that dies during
    // `AEEMod_Load` is exactly the run whose tree a reader wants, and the loader's call into the
    // module is a downcall like any other. An unarmed run folds nothing, so this is free when the
    // flag was not given.
    rt.profile.set(flame);
    // Arming before the load means the loader's own copy of the image is in the record too. A field
    // that was never written and a field that was written once and then overwritten look the same
    // from a crash site, and differ completely in what to do about it.
    for each in watch.iter().copied() {
        if let (addr, None) = each {
            rt.cpu.mem.watch_word(addr);
        }
    }
    rt.cpu.mem.watch_stop = watch_stop;
    rt.cpu.mem.watch_chain = watch_chain;
    // A run that named a number meant it. Otherwise this is a pack being *played* — which a scripted
    // `--wait`/`--key` run is just as much as a windowed one, since both drive the same guest through
    // the same frames — so it gets the bound measured from the corpus rather than the 2-million probe
    // default a unit test wants. A bound that depends on whether a window is open leaves the scripted
    // form under a limit smaller than one frame of guest work, so the same pack played two ways
    // answers two different questions. See `GAME_STEP_LIMIT`.
    rt.step_limit = steps.unwrap_or(quoin::runtime::GAME_STEP_LIMIT);
    rt.step_ceiling = steps_ceiling.unwrap_or(quoin::runtime::GAME_STEP_CEILING);
    // Off unless asked for: a pack that returns from every function it was given has not done
    // anything this bound has to say about it, and a default would be one more number the corpus has
    // to be measured against.
    rt.session_ceiling = session_ceiling.unwrap_or(u64::MAX);
    // A run told where to send its requests gets the one thread and the one socket this host has.
    // Without the flag `rt.net` stays `None` and nothing in `src/net.rs` is reachable, which is what
    // keeps an ordinary run free of the wall clock. An address that cannot even be parsed is worth
    // failing the run for: the flag would otherwise do nothing at all, silently, which is the one
    // outcome a developer cannot debug.
    if net_proxy.is_some() && net_script.is_some() {
        return Err(
            "--net-proxy and --net-script are two different network backends; choose one".into(),
        );
    }
    if let Some(addr) = net_proxy {
        let proxy = quoin::net::Proxy::new(&addr)?;
        // The same address becomes the socket layer's policy, so a pack that writes its own protocol
        // over a raw `ISocket` goes through the proxy too -- not only the requests that were made
        // through `IWeb`. An address this cannot read as one socket (`Proxy::new` may have resolved a
        // name) leaves sockets on the loopback model, and says so rather than pretending.
        match addr.parse::<std::net::SocketAddr>() {
            Ok(target) => {
                rt.host_sockets.set_policy(quoin::socket::Policy {
                    real: true,
                    proxy: Some(target),
                });
                println!("network: guest sockets connect through {target}");
            }
            Err(_) => println!(
                "network: {addr} is not a socket address here, so guest sockets stay on the loopback model"
            ),
        }
        // Said out loud, because "did anything actually go anywhere" is the first question an empty
        // `net` log raises — and a run whose pack never asked is a different answer from a run whose
        // flag was swallowed.
        println!("network: the guest's requests go through {}", proxy.at);
        rt.net = Some(proxy);
    } else if let Some(path) = net_script {
        let proxy = quoin::net::Proxy::from_script(std::path::Path::new(&path))?;
        println!(
            "network: the guest's requests are answered by the local script {}",
            proxy.at
        );
        // A script's `SOCKET` lines answer raw sockets: a connect they name never leaves the process
        // and its reply comes from the file. With none, guest sockets stay on the loopback model —
        // said out loud for the same reason the proxy case is, because the two are different answers
        // to "did anything actually go anywhere".
        let sockets = proxy.sockets().clone();
        if sockets.is_empty() {
            println!("network: guest sockets stay on the loopback model (the script has no SOCKET lines)");
        } else {
            println!("network: guest sockets connect through the script's SOCKET lines");
        }
        rt.host_sockets.set_script_routes(sockets);
        rt.net = Some(proxy);
    }
    // A null resource file name means "the app's own", which on a handset is the pack beside the
    // image, so mount what is there and say what was found.
    let mounted = rt.resources.mount_beside(std::path::Path::new(path));
    if !mounted.is_empty() && verbose {
        println!(
            "resources mounted from beside the .mod: {}",
            mounted.join(", ")
        );
    }
    let files = rt.volume.mount_beside(std::path::Path::new(path));
    // Where the app's own writes go. The default is a per-pack directory beside the emulator, because
    // a save that vanishes with the run is exactly what "the game keeps starting over" reads as.
    // `--save-dir DIR` puts it somewhere else and `--save-dir none` turns it off, which is what a
    // regression run wants: a run with a home directory is not reproducible from the same starting
    // state, so a baseline taken with one would only describe the save it started from.
    let home = match save_dir.as_deref() {
        Some("none") => None,
        Some(dir) => Some(std::path::PathBuf::from(dir)),
        None => quoin::library::default_save_dir(std::path::Path::new(path)),
    };
    if let Some(dir) = &home {
        let loaded = rt.volume.persist_to(dir);
        println!(
            "app files: keeping what the app writes in {} ({} entr{} loaded from it)",
            dir.display(),
            loaded.len(),
            if loaded.len() == 1 { "y" } else { "ies" }
        );
    }
    if !files.is_empty() && verbose {
        let shown = if files.len() > 6 {
            format!("{}, …", files[..6].join(", "))
        } else {
            files.join(", ")
        };
        println!(
            "the app's own directory ({} file(s)): {}",
            files.len(),
            shown
        );
    }
    // The first thing a module is given is the screen it will lay its art out for, and the one thing
    // a shipped pack cannot be trusted to have guessed right: built for another size, it draws
    // misaligned rather than failing, so the number goes on the record before the load does.
    println!(
        "device: {}x{} {}bpp, model {:?} — a pack built for another screen wants --device{}",
        rt.device.width,
        rt.device.height,
        rt.device.color_depth,
        rt.device.model_name,
        // Say out loud when the advertised depth is not this host's screen, because the frame under it
        // did not change: a run that draws wrong here has shown a code path, not a pixel format.
        if rt.device.color_depth != quoin::display::NATIVE_DEPTH {
            format!(
                " (advertised only: the frame stays {}bpp RGB565)",
                quoin::display::NATIVE_DEPTH
            )
        } else {
            String::new()
        }
    );
    // A module that never comes up is a run whose diagnosis is the report, not the sentence the
    // error carries: the call log up to the fault, the registers it faulted with, and the interface
    // the run ended inside. A `?` here would take all three out, which is exactly what a pack that
    // stops before its applet leaves a reader with.
    let loaded = match rt.load(&image, thumb) {
        Ok(loaded) => loaded,
        Err(e) => return Err(failed(&rt, verbose, bmp.as_deref(), &debug, e)),
    };
    // `--lib` images are read before any of them is loaded, because `--jit-arena auto` sizes the
    // code cache from the total bytes of module code and has to be applied before the first library
    // entry point runs: setting the ceiling flushes the cache, which is only free while it is empty.
    let lib_images: Vec<(&String, Vec<u8>)> = libraries
        .iter()
        .map(|lib| {
            std::fs::read(lib)
                .map(|bytes| (lib, bytes))
                .map_err(|e| format!("--lib {lib}: {e}"))
        })
        .collect::<Result<_, _>>()?;
    if let Some(choice) = jit_arena {
        let code_bytes = u64::from(loaded.len)
            + lib_images
                .iter()
                .map(|(_, bytes)| bytes.len() as u64)
                .sum::<u64>();
        if let Some(line) = rt.cpu.size_jit_arena(choice, code_bytes) {
            println!("{line}");
        }
    }
    for (lib, bytes) in &lib_images {
        rt.load_library(bytes, false)
            .map_err(|e| format!("--lib {lib}: {e}"))?;
    }
    // The rest of the bundle, so a launcher shim can start the class it names: `Refresher.mod`'s whole
    // `EVT_APP_START` is `StartApplet(monopoly.mod's class)`. An explicit `--lib` still covers a module
    // filed outside the bundle; this only adds what sits beside the pack.
    let siblings = quoin::library::load_sibling_modules(&mut rt, std::path::Path::new(path));
    for loaded in &siblings {
        if verbose || loaded.contains("(skipped)") {
            println!("bundle module: {loaded}");
        }
    }
    for each in watch.iter().copied() {
        if let (addr, Some(len)) = each {
            rt.cpu.mem.watch_span(addr, len);
        }
    }
    // Breakpoints are resolved here rather than while the command line was being read, because the
    // address someone names is usually an offset into the image and the image only now exists. A name
    // that resolves to nothing is an error and not a silent skip: a run that never stops where it was
    // asked to would otherwise look like a run that did not get there.
    //
    // `ADDR,N` waits for the N-th arrival, because the call that explains a pack is often the third one
    // through a function that runs once per record. A bare `ADDR` is `ADDR,1`.
    //
    // `ADDR,r0=0` waits instead for the arrival where a register holds a value. Counting is the wrong
    // instrument when the function runs thousands of times and only one call is the failing one: each
    // step of that binary search is a whole boot of the pack.
    for text in &break_at {
        let mut fields = text.split(',');
        let what = fields.next().unwrap_or("").trim();
        let Some(addr) = symbol::parse(rt.module.as_ref(), what) else {
            return Err(format!(
                "--break {text:?}: not an address this host knows — try 0x…, module+0x…, heap+0x… \
                 or IShell::CreateInstance, each optionally followed by ,N to stop at the N-th \
                 arrival or ,r0=0x… to stop where a register holds a value"
            ));
        };
        let mut after = 1;
        let mut when = None;
        for field in fields.map(str::trim).filter(|f| !f.is_empty()) {
            let (name, want) = match field.split_once('=') {
                Some(half) => half,
                None => match symbol::number(field).filter(|n| *n >= 1) {
                    Some(n) => {
                        after = n;
                        continue;
                    }
                    None => {
                        return Err(format!(
                            "--break {text:?}: {field:?} is neither an arrival count (a number from 1 \
                             up) nor a register test (r0=… through r12=…, sp=…, lr=…)"
                        ));
                    }
                },
            };
            let Some(reg) = register_number(name) else {
                return Err(format!(
                    "--break {text:?}: {name:?} is not a register — r0…r12, sp and lr are"
                ));
            };
            let Some(val) = symbol::number(want) else {
                return Err(format!(
                    "--break {text:?}: no value after {name:?}= — 0x… or decimal"
                ));
            };
            when = Some(quoin::cpu::RegTest { reg, val });
        }
        rt.cpu.break_at.push(quoin::cpu::Break {
            addr,
            after,
            seen: 0,
            when,
        });
    }
    println!(
        "{}: {} bytes mapped at {:#010x}, helper={:#010x}, IShell={:#010x}",
        abi::MODULE_ENTRY,
        loaded.len,
        loaded.base,
        rt.hle.helper_table_addr(),
        rt.shell
    );

    let outcome = match rt.run() {
        Ok(outcome) => outcome,
        // Same reason as the load above: the fault, its registers and the call it happened inside
        // are the run's answer, and a `?` would drop them on the floor.
        Err(e) => return Err(failed(&rt, verbose, bmp.as_deref(), &debug, e)),
    };
    // Whether the applet phase still happens after this phase — see [`start_after_load`]. Whether it was
    // asked for at all is a different question, and the two answers are needed apart below.
    let load_stop = matches!(outcome, Outcome::Stopped { .. });
    // A load that spent its instruction budget did not return, so there is no `IModule*` yet. It is
    // not the module refusing to publish one: a self-unpacking loader in this corpus needs 4–18
    // million instructions before `AEEMod_Load` returns, and a headless run with a small ceiling
    // cuts it off mid-unpack. Starting an applet on that machine would report both a false error and
    // a second, unrelated machine state.
    let load_unfinished = matches!(outcome, Outcome::Stepped { .. });
    let asked_start = start.is_some();
    let mut code = match outcome {
        Outcome::Halted { retval, steps } => {
            println!(
                "AEEMod_Load returned {:#010x} ({}) after {} guest instructions",
                retval,
                result_name(retval),
                steps
            );
            retval
        }
        Outcome::Stepped { steps, spun } => {
            // `hard` is the smaller of the per-function ceiling and the whole-run ceiling
            // (`runtime/mod.rs::run_bounded`), so the flag that actually ended this phase is the one a
            // person has to raise. Naming `--steps-ceiling` for a run stopped by
            // `--session-ceiling` sends the reader to a number that will not move the result.
            let ceiling = if steps >= rt.session_ceiling {
                "--session-ceiling"
            } else if spun {
                "--steps-ceiling"
            } else {
                "--steps"
            };
            println!(
                "{}",
                if spun {
                    format!(
                        "still running after {steps} instructions — it kept reaching the host, so \
                         this is a long loop rather than a stuck one; raise {ceiling}"
                    )
                } else {
                    format!(
                        "still running after {steps} instructions without reaching the host — \
                         raise --steps, or use --trace to see where"
                    )
                }
            );
            1
        }
        Outcome::Cancelled { pc, steps } => {
            // Not a failure and not a finding: somebody asked this run to stop, so the exit code says
            // nothing went wrong. What is worth printing is where the guest had reached, because the
            // reason a person cancels is that the picture stopped moving — and that question has an
            // answer only if the run says where it was spinning.
            println!(
                "\ncancelled after {steps} guest instruction(s): the guest was at {}",
                symbol::symbolize(&rt.hle, rt.module.as_ref(), pc)
            );
            0
        }
        Outcome::Stopped { why, steps } => {
            // Stopped on purpose, so this is not a failure: the run reached the address that was asked
            // about, and everything below is the machine at that moment rather than after it.
            let at = match why {
                quoin::cpu::StopWhy::Watch(addr) => {
                    format!(
                        "{} was touched",
                        symbol::symbolize(&rt.hle, rt.module.as_ref(), addr)
                    )
                }
                quoin::cpu::StopWhy::Break(addr) => {
                    // Say which arrival this was: a run stopped at "the third time through" answers a
                    // different question than one stopped at the first, and the reader cannot tell
                    // otherwise. With a register test, `seen` is how many arrivals it passed over on
                    // the way to the one that held the value.
                    let (seen, awaited) = rt
                        .cpu
                        .break_at
                        .iter()
                        .find(|b| b.addr == addr)
                        .map(|b| {
                            (
                                b.seen,
                                match b.when {
                                    None => format!("of arrival #{}", b.after.max(1)),
                                    Some(t) => format!(
                                        "waiting for {} == {:#010x}",
                                        REG_NAMES.get(t.reg as usize).copied().unwrap_or("?"),
                                        t.val
                                    ),
                                },
                            )
                        })
                        .unwrap_or((1, String::new()));
                    format!(
                        "about to run {} (arrival {seen} {awaited})",
                        symbol::symbolize(&rt.hle, rt.module.as_ref(), addr)
                    )
                }
            };
            println!("\nstopped after {steps} guest instruction(s): {at}");
            report_crash(&rt);
            0
        }
    };
    let module = rt.created_module().map_err(|e| e.to_string())?;
    if module != 0 {
        println!(
            "IModule* = {:#010x} (vtbl {:#010x}, refs {})",
            module,
            read_word(&rt, module)?,
            read_word(&rt, module + 4)?
        );
    }

    // A load phase that stopped on purpose, or ran out of instructions before returning, ends the
    // run before `--start`. Saying so out loud is the other half: a session that asked for an applet
    // and got none must not look like a flag ignored.
    let (next, skipped) = start_after_load(start, load_stop || load_unfinished);
    start = next;
    if skipped {
        // Name everything the dropped phase would have consumed, or the run accepts a `--window` and a
        // `--key` and then does nothing with either — the accepted-then-silent shape, one flag further
        // along from the one this gate fixed.
        let mut dropped: Vec<&str> = Vec::new();
        if gui.is_some() {
            dropped.push("--window");
        }
        if serve.is_some() {
            dropped.push("--serve");
        }
        if !cmds.is_empty() {
            dropped.push("--key/--event/--pointer/--tap/--pad/--wait");
        }
        if run_for > 0 {
            dropped.push("--run-for");
        }
        println!(
            "--start skipped: {}{}",
            start_skip_reason(load_unfinished),
            if dropped.is_empty() {
                String::new()
            } else {
                format!("; also dropped: {}", dropped.join(" "))
            }
        );
    }

    if let Some(which) = start {
        if module == 0 {
            return Err("--start needs a module that published an IModule*".into());
        }
        // `auto` asks the module for each class id its own image contains and starts the first one it
        // builds, so a session does not have to begin by being told the answer.
        let (cls, rv) = match which {
            Start::Cls(cls) => (
                cls,
                rt.start_app(cls, None)
                    .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?,
            ),
            Start::Auto => {
                let found = rt
                    .start_auto()
                    .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?;
                if found.cls == 0 {
                    // "Claims none of them" is the whole answer only when the module turned every id
                    // down. One it took and then failed to build is a different fact — and the probe
                    // kept it, so it comes out on this line instead of being said as a shrug.
                    let why = match found.note {
                        Some(note) => format!(
                            "the module built no applet for any of the {} class ids in its own \
                             image; {note}",
                            found.tried
                        ),
                        None => format!(
                            "the module claims none of the {} class ids in its own image",
                            found.tried
                        ),
                    };
                    return Err(failed(&rt, verbose, bmp.as_deref(), &debug, why));
                }
                println!(
                    "--start auto: {:#010x} is this module's applet class ({} class id(s) asked first)",
                    found.cls,
                    found.tried - 1
                );
                // An owned class whose applet then could not be built is still an answered question,
                // but the ownership answer is not a started applet. `note` means the start stopped
                // partway — a guest fault or a refusal — so that reason ends the run here rather
                // than becoming a footnote under a success line, and the scripted keys below are
                // not delivered to a half-built applet.
                if let Some(why) = found.note {
                    return Err(failed(&rt, verbose, bmp.as_deref(), &debug, why));
                }
                let started = start_counts(found.applet, rt.applet);
                (
                    found.cls,
                    if started {
                        quoin::abi::generated::AEE_SUCCESS
                    } else {
                        quoin::abi::generated::AEE_EFAILED
                    },
                )
            }
        };
        println!(
            "StartApplet({:#010x}) -> {} (applet {:#010x})",
            cls,
            result_name(rv),
            rt.applet
        );
        // The applet's entry points live in its vtable rather than at a fixed place in the image —
        // the pack builds them at run time and hands the addresses over through `IExtension`. The
        // slot it hands over is itself a hop: `0x20025ed8` is `ldr r12, [r0, #0x18]; bx r12` in ARM
        // (a managed-object trampoline), so the address worth disassembling is the one stored inside
        // the applet. Printing both is what makes a misbehaving callback reachable at all.
        if let Ok(slot) = rt.guest_slot(rt.applet, "IApplet", "HandleEvent") {
            println!("  IApplet::HandleEvent = {slot:#010x}");
            if let Ok(word) = rt.cpu.mem.read_u32(rt.applet + 0x18) {
                println!("    *[applet+0x18] = {word:#010x}");
            }
        }
        // Keys and events go to an applet. Delivering them to none would bury the reason the start
        // failed under a second error about the shell having nothing running.
        if rv != quoin::abi::generated::AEE_SUCCESS {
            return Err(failed(
                &rt,
                verbose,
                bmp.as_deref(),
                &debug,
                format!("--start {:#010x} created no applet", cls),
            ));
        }
        // Scripted input is *posted* rather than delivered, so a session goes in through the same
        // door a real key goes in through — and so that whatever the applet schedules for itself on
        // the strength of those keys is heard by the same pump below.
        //
        // Each `--wait` is its own turn of the loop and the summary at the end describes the whole
        // session, so the turns are folded in rather than thrown away. Reporting only the last pump
        // once made a session that had delivered twenty-one keys look like it had delivered three.
        let mut session = Dispatched::default();
        // Once the applet has closed there is nobody left to take an input event, and the host answers
        // that by raising `no applet is running` — which is the documented and right thing when a *pack*
        // asks for it, because a program posting at its own dead session has a bug worth naming (see
        // the applet-shutdown contract). It is the wrong thing when the one pressing the key
        // is this driver: a pack may draw a frame, print its own memory report and close its session on
        // purpose, and then the scripted `--key` that follows ends a finished run with an error and
        // exit code 1 — which reads, on the frame-capture screen,
        // exactly like a pack that failed to start. A handset with nothing running does not fail the
        // person holding the keyboard; the key goes nowhere. So the rest of the scripted input is
        // skipped, said once on the default output rather than behind `--verbose`, and the run is allowed
        // to end the way the pack ended it. `--wait` still runs: it is the clock, not an input.
        let mut ignored = false;
        for cmd in &cmds {
            if rt.applet == 0 && !matches!(cmd, Cmd::Wait(_)) {
                ignored = note_ignored(ignored, rt.idle);
                // Do not skip the clock with the delivery. Three of these commands carry a hold window
                // of their own — `--key`, `--tap`, and a pad `Tap` all pump `hold` milliseconds between
                // the press and the release so a polling pack can see them in different turns — and
                // dropping that too would make a run's virtual time depend on when the applet happened
                // to close. That is a different, and much worse, change than simply not delivering:
                // it would move the timing of everything after the close.
                if matches!(
                    cmd,
                    Cmd::Key(_) | Cmd::Tap { .. } | Cmd::Pad(PadInput::Tap { .. })
                ) {
                    session.accumulate(
                        rt.pump(hold)
                            .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?,
                    );
                }
                continue;
            }
            // Re-arm the message. `--serve` adopts applets in one process, and each session that ends
            // deserves to say so; leaving the flag set would make the second close silent and turn the
            // first "this session is over" into a claim about a run that kept going.
            ignored = false;
            match cmd {
                Cmd::Event { evt, wp, dwp } => rt.post_event(*evt, *wp, *dwp),
                Cmd::Pointer { evt, x, y } => rt
                    .post_pointer(*evt, *x, *y)
                    .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e.to_string()))?,
                Cmd::Tap { x, y } => {
                    // Down, a moment with it down, up — the same reason `--key` holds a key down
                    // rather than tapping it in one turn: an app that reads which contacts are
                    // *currently* down sees nothing at all when the press and the release arrive
                    // together, and says so by simply not moving.
                    rt.post_pointer(abi::generated::EVT_POINTER_DOWN, *x, *y)
                        .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e.to_string()))?;
                    session.accumulate(
                        rt.pump(hold)
                            .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?,
                    );
                    if rt.applet == 0 {
                        ignored = note_ignored(ignored, rt.idle);
                        continue;
                    }
                    rt.post_pointer(abi::generated::EVT_POINTER_UP, *x, *y)
                        .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e.to_string()))?;
                }
                Cmd::Key(code) => {
                    // Down, a moment with it down, up. The middle is not decoration: an app that
                    // keeps a mask of the keys currently held cannot see a tap whose press and
                    // release land in the same turn, so the pump is what makes the key *noticed*
                    // rather than merely delivered.
                    rt.post_key_press(*code);
                    session.accumulate(
                        rt.pump(hold)
                            .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?,
                    );
                    // The guard is not paranoia about the loop above. The pump that just ran is
                    // where the pack can decide to quit — closing its own applet inside the first
                    // `--wait` — and if it did, this release would be the queued job left over
                    // when the run reports "1 queued task", the one delivery that reaches
                    // `send_event` with nothing running and ends a finished session with
                    // `no applet is running`.
                    if rt.applet == 0 {
                        ignored = note_ignored(ignored, rt.idle);
                        continue;
                    }
                    rt.post_key_release(*code);
                }
                Cmd::KeyDown(code) => rt.post_key_press(*code),
                Cmd::KeyUp(code) => rt.post_key_release(*code),
                // A pad control moved here is only a state change and the notification that goes with
                // it; the pack is reached by the next turn of the loop, the same turn that delivers a
                // queued key. That is the reproducibility, and it is why nothing below dispatches.
                Cmd::Pad(input) => match *input {
                    PadInput::Button { button, down } => rt
                        .pad_press(button, down)
                        .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?,
                    PadInput::Axis { axis, value } => rt
                        .pad_axis(axis, value)
                        .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?,
                    PadInput::Tap { button } => {
                        // Two turns, for the reason `--hold` and `/advance` carry: a pack that polls
                        // which buttons are down sees nothing at all in a press that opens and closes
                        // inside one dispatch, so the release has to wait for a turn that ran with
                        // the button down. With no pump after this, the one `--run-for` gives is what
                        // releases it.
                        rt.pad_press(button, true)
                            .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?;
                        session.accumulate(
                            rt.pump(hold)
                                .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?,
                        );
                        // The same mid-window close as `Cmd::Key`: notifying a control whose applet
                        // has gone is how a release meant for one session reaches the next.
                        if rt.applet == 0 {
                            ignored = note_ignored(ignored, rt.idle);
                            continue;
                        }
                        rt.pad_press(button, false)
                            .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?;
                    }
                },
                // Let the app run to the state a real player would have reached before pressing
                // anything. A key posted up front is answered by the title or intro screen, which
                // discards it, and no later state ever sees it again — exactly as on a handset.
                Cmd::Wait(ms) => {
                    session.accumulate(
                        rt.pump(*ms)
                            .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?,
                    );
                }
            }
        }
        // Either look at it or record it. A window *is* a dispatch loop — the shell's own, driven by
        // the wall clock and a keyboard — so it stands in for the scripted pump rather than adding to
        // it. Scripted `--wait`/`--key` above still counts: they only queued work, and the loop
        // delivers it through the same door a real press does.
        // A window is titled by the pack, because four builds of one game look identical otherwise — and
        // by the name the pack's own manifest carries, because what is on disk is `tank.mod` while what
        // the player remembers buying is the title inside it. The emulator's own name leads, so a taskbar
        // full of windows still says which of them this host made.
        let pack = path.rsplit(['/', '\\']).next().unwrap_or(path);
        let title = format!(
            "{} - {}",
            quoin::APP_NAME,
            rt.applet_name().unwrap_or_else(|| pack.to_string())
        );
        // The control interface comes up before the front end that will poll it, so a caller that
        // connects while the run is starting is already talking to something. Nothing listens unless
        // it was asked for: a port opened uninvited is a surprise, not a feature.
        let mut control = match &serve {
            Some(addr) => {
                let control = quoin::control::Control::bind(addr)
                    .map_err(|e| format!("--serve {addr}: {e}"))?;
                println!(
                    "control: listening on http://{} — GET / lists the verbs",
                    control
                        .local()
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| format!("127.0.0.1:{}", control.port()))
                );
                Some(control)
            }
            None => None,
        };
        if let Some(permille) = quoin::session::clock_speed() {
            println!(
                "clock: {:.2}x — virtual time follows the wall clock; a stall is paid back only at 1x",
                permille as f32 / 1000.0
            );
        }
    // `--fps` names the HUD line, and only the egui window draws one. A build without the window
    // still parses and validates the flag; reading it once is what keeps that from turning into
    // four "never used" warnings that would hide a real one.
    #[cfg(not(feature = "ui"))]
    let _ = show_fps;

        let pumped = match gui {
            #[cfg(feature = "ui")]
            Some(Gui::Egui) => {
                // The front end moves the runtime in and moves it back out, because a window that
                // has closed cannot also be holding the thing the report is read from. It hands the
                // reason out with it: a window the person closed and one that closed itself because
                // the guest faulted look identical from this side, and the report below says
                // different things about them. See [`quoin::session::Finished::why`].
                let done = quoin::egui_window::show(
                    rt,
                    &title,
                    skin.as_deref().unwrap_or("graphite"),
                    soundfont.as_deref(),
                    audio_backend,
                    control.take(),
                    show_fps,
                );
                rt = done.rt;
                match done.error {
                    Some(e) => {
                        return Err(failed_ended(
                            &rt,
                            verbose,
                            bmp.as_deref(),
                            &debug,
                            e,
                            done.why,
                        ))
                    }
                    None => done.pumped,
                }
            }
            // The parser accepts `--window egui` either way, so the answer here is the real reason
            // this build cannot show one — not "unknown argument", which would send a person
            // looking at their spelling. `--window raw` needs no toolkit and is still available.
            #[cfg(not(feature = "ui"))]
            Some(Gui::Egui) => return Err(
                "--window egui is not in this build: it was made without the `ui` feature, which \
                 is what links the window toolkit. Use --window raw, or rebuild with --features ui."
                    .to_string(),
            ),
            Some(Gui::Raw) => {
                // Same shape as the egui window: the machine goes in, the report comes out of it.
                let done = quoin::window::show(
                    rt,
                    &title,
                    soundfont.as_deref(),
                    audio_backend,
                    control.take(),
                );
                rt = done.rt;
                match done.error {
                    Some(e) => {
                        return Err(failed_ended(
                            &rt,
                            verbose,
                            bmp.as_deref(),
                            &debug,
                            e,
                            done.why,
                        ))
                    }
                    None => done.pumped,
                }
            }
            None => {
                // One turn of the shell's own loop. With no `--run-for` this drains exactly what is
                // already queued and lets no time pass; with one it keeps going until nothing is
                // left to run, which is what an applet that animates or defers its drawing needs
                // before it has drawn anything. There is nobody here to cancel or quit a run, so
                // this path keeps the plain `failed`.
                let mut pumped = rt
                    .pump(run_for)
                    .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?;
                // `--serve` without a window: the guest moves only when a caller asks it to, so a
                // session driven from outside is as reproducible as one driven by `--key`. Nothing
                // below lets time pass on its own.
                if let Some(mut control) = control.take() {
                    pumped.accumulate(
                        serve_loop(&mut control, &mut rt)
                            .map_err(|e| failed(&rt, verbose, bmp.as_deref(), &debug, e))?,
                    );
                }
                pumped
            }
        };
        // The scripted turns above drove the same loop, so their work is part of the session the
        // line below reports.
        session.accumulate(pumped);
        let pumped = session;
        if !cmds.is_empty() || run_for > 0 || gui.is_some() {
            let mut waiting = Vec::new();
            if !rt.timers.is_empty() {
                waiting.push(format!("{} timer(s) still armed", rt.timers.len()));
            }
            if !rt.queue.is_empty() {
                waiting.push(format!("{} job(s) still queued", rt.queue.len()));
            }
            // A deferred key release is not in that queue, and a report that reads only the queue
            // would say "nothing left to run" about a key the applet has not been told came up.
            if !rt.pending_release.is_empty() {
                waiting.push(format!(
                    "{} key release(s) still waiting on their press",
                    rt.pending_release.len()
                ));
            }
            println!(
                "dispatch: {} event(s){}, {} callback(s), {} ms of virtual time ({})",
                pumped.events,
                if pumped.unhandled == 0 {
                    String::new()
                } else {
                    format!(", {} declined by the applet", pumped.unhandled)
                },
                pumped.callbacks,
                pumped.ms,
                if pumped.idle && rt.idle {
                    // The applet closed with `bReturnToIdle`: on a handset the shell would be
                    // running its idle applet now. This host has none, so the session ends — but it
                    // ends the way the pack asked it to, not as a failure.
                    "returned to the shell's idle applet".to_string()
                } else if pumped.idle {
                    "idle, nothing left to run".to_string()
                } else {
                    format!("{}, so the window ran out first", waiting.join(", "))
                }
            );
        }
        if let Some(p) = &bmp {
            write_bmp(&rt, p)?;
        }
        report_threads(&rt);
        if verbose || !rt.log_args.is_empty() {
            print_call_log(&rt);
        }
        report_display(&rt, verbose);
        report_heap(&rt);
        report_watch(&rt);
        report_break(&rt);
        report_samples(&rt);
        report_flame(&rt, flame);
        // A clean run's `--disasm` and `--dump`, answered here rather than in `failed`: this is the
        // exit that was not an error, and asking what a run ended *on* is as useful as asking what it
        // died on.
        report_disasm(&rt, debug.disasm.as_ref());
        report_dump(&rt, debug.dump.as_ref());
        report_trace(&rt);
        if debug.coverage {
            report_coverage(&rt);
        }
        if let Some(d) = &dump_audio {
            write_audio(&rt, d)?;
        }
        if let Some(d) = &dump_wav {
            write_wav(&rt, d, soundfont.as_deref())?;
        }
        report_audio(&rt, verbose);
        code = rv;
    } else {
        // Three flags write about an applet's session, and two different things reach here. Never
        // passing `--start` is the reader's mistake and has to come back as an error. Passing it and
        // having the load run stop first is not: the run then already said it stopped, and telling that
        // person to add a flag they added is noise. Both mean no file, so both say so — and accepting
        // the flag and then writing nothing is the shape this host refuses elsewhere.
        let no_run = "nothing names a clip without a run";
        for (flag, taken, because) in [
            ("--bmp", &bmp, "nothing draws without an applet"),
            ("--dump-audio", &dump_audio, no_run),
            ("--dump-wav", &dump_wav, no_run),
        ] {
            let Some(p) = taken else { continue };
            if asked_start {
                println!("{flag} {p} not written: the load run stopped before any applet ran");
            } else {
                return Err(format!("{flag} {p} needs --start, since {because}"));
            }
        }
        if verbose || !rt.log_args.is_empty() {
            print_call_log(&rt);
        }
        // Refusals are said here too: without `--start` there is no screen to report, but the pack's
        // load-time code still asks for files, and a run that was refused something should not have to
        // be told to pass a second flag to find that out.
        report_refusals(&rt);
        // The same rule covers the instruments a *load* run can answer. Each of these returns without
        // printing unless it was asked for on the command line, so the only cost is volume a person
        // requested; and a `--break` that fired while `AEEMod_Load` ran is now the end of the run, so
        // this is the only place its arrival tally and its `--dump` can come out.
        report_watch(&rt);
        report_break(&rt);
        report_samples(&rt);
        report_flame(&rt, flame);
        report_disasm(&rt, debug.disasm.as_ref());
        report_dump(&rt, debug.dump.as_ref());
        report_trace(&rt);
    }
    Ok(if code == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Whether the applet phase runs after `AEEMod_Load`, and whether the caller has to say it was cut
/// short — the second value is true only when a `--start` was asked for and is being dropped.
///
/// A load phase that ended before `AEEMod_Load` returned ends the run for two different reasons. A
/// deliberate stop keeps its instruments armed: `--break` stays awaited once its arrival count is
/// reached (`src/cpu/mod.rs:335`), and a `--watch-stop` byte trips on every later access to it. So a
/// load-phase stop followed by `--start` stops again at the first touch of the applet's own, and the
/// reports printed once at the end — a `--dump` of a register above all — describe *that* machine
/// while the line above them still reads "stopped". Starting anyway bought nothing and made the one
/// report whose honesty depends on where the machine stood quietly false.
///
/// The other case is a load that spent its instruction budget and never returned. There is no
/// `IModule*` to start from, and blaming the module for not publishing one is the wrong diagnosis:
/// self-unpacking loaders in the corpus need millions of instructions before they return. Both cases
/// drop the applet phase; [`start_skip_reason`] keeps the two explanations apart.
fn start_after_load(start: Option<Start>, load_incomplete: bool) -> (Option<Start>, bool) {
    if load_incomplete {
        (None, start.is_some())
    } else {
        (start, false)
    }
}

/// The line that explains a dropped applet phase, kept apart from the `--start skipped` prefix so
/// the two reasons cannot be swapped by a caller that only knows one boolean.
/// Whether `--start auto` counts as having started something.
///
/// A class the probe found and an object the module built is not enough on its own: a pack that
/// builds its applet and then closes it has started nothing a person can look at. What is enough is
/// that an applet is *running* afterwards — and it need not be the one the probe found. Refresher's
/// whole `EVT_APP_START` is `CloseApplet` and `StartApplet(monopoly.mod's class)`, so the applet that
/// ends up running belongs to a different module than the one that was launched, and comparing the
/// two made a working run report `--start … created no applet`.
fn start_counts(found_applet: u32, running: u32) -> bool {
    found_applet != 0 && running != 0
}

fn start_skip_reason(load_unfinished: bool) -> &'static str {
    if load_unfinished {
        "AEEMod_Load never returned: the load phase spent its instruction budget. A pack that \
         unpacks itself in AEEMod_Load needs more instructions than that; raise --steps / \
         --steps-ceiling / --session-ceiling, or leave them at the defaults"
    } else {
        "the load run stopped where it was asked to, and a second phase would report a machine \
         that is not the one stopped at"
    }
}

/// Serve the control interface until a caller quits, letting the guest run exactly as far as it is
/// asked to.
///
/// This is the headless half of the interface. There is no wall clock here, so `/advance` is the
/// only thing that moves the guest — which is what keeps a session driven from outside as
/// reproducible as one driven by `--key`, and the reason `--serve` is a mode of its own rather than
/// something the scripted path grew.
fn serve_loop(
    control: &mut quoin::control::Control,
    rt: &mut Runtime,
) -> Result<Dispatched, quoin::runtime::Error> {
    let mut out = Dispatched::default();
    loop {
        let asked = control.poll(rt);
        if asked.quit {
            return Ok(out);
        }
        if asked.advance_ms > 0 {
            out.accumulate(rt.pump(asked.advance_ms)?);
        } else {
            // Nothing to do and nobody has asked for time to pass: a shell waiting on a client
            // sleeps rather than spinning, and the sleep costs no virtual time — the clock the guest
            // sees only moves in `pump`.
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
}

/// The dispatch trace, the screen report and the screenshot are what makes a partial run
/// diagnosable, so they ride out with the error instead of being skipped by it. A run that died
/// halfway through its first frame is exactly the case where the picture matters most.
///
/// This is the shape every path but the windowed one uses, and the reason it needs no `why` of its
/// own: with no window and no control interface serving `quit`, there is nobody who can end a run
/// except the guest or a fault. See [`failed_ended`] for the two front ends, where that stops being
/// true.
fn failed(
    rt: &Runtime,
    verbose: bool,
    bmp: Option<&str>,
    debug: &Debug,
    e: impl std::fmt::Display,
) -> String {
    failed_ended(rt, verbose, bmp, debug, e, quoin::session::End::Finished)
}

/// The same report, plus the one thing it used to be unable to say.
///
/// `why` is how the run ended, and it matters here because a windowed session is ended by the person
/// closing the window far more often than by a fault — and when the ask arrives while a turn is still
/// executing, that turn ends as an error. The console then printed a full crash report for a run the
/// reader had just chosen to stop, and the only thing it had to go on was a parenthesis admitting it
/// could not tell what it was looking at. Nothing here is suppressed: a refusal is a refusal and the
/// evidence is the evidence. What changes is that the report says which of the two it is.
fn failed_ended(
    rt: &Runtime,
    verbose: bool,
    bmp: Option<&str>,
    debug: &Debug,
    e: impl std::fmt::Display,
    why: quoin::session::End,
) -> String {
    if verbose || !rt.log_args.is_empty() {
        print_call_log(rt);
    }
    // The one-line display summary comes out either way, because a run that failed is exactly when
    // what it managed to draw is wanted; only `--verbose` adds the text runs and the notes on top.
    report_display(rt, verbose);
    // A refusal is worth more on the path that faulted than on the clean one, which is the whole
    // reason this sits here as well: a pack that took a NULL and dereferenced it three hundred
    // million instructions later ends its run exactly here, with nothing else to say.
    report_heap(rt);
    // A watchpoint was asked for on the command line, and a run that ends in a fault never reaches
    // the report at the end of `run`, so this is the only place its answer can come out. Volume on
    // the command line is opt-in, and a diagnosis the caller requested is not volume.
    report_watch(rt);
    report_break(rt);
    // And the samples: the run that faulted is the one whose tail matters most, and a ring kept for
    // twenty minutes of play has nowhere else to come out.
    report_samples(rt);
    // The run that faulted is the one whose tree is worth most, and it never reaches the report at
    // the end of `run` — so, like the samples, the tree comes out on this path too.
    report_flame(rt, rt.profile.on());
    report_crash(rt);
    // `--disasm` with no address means "wherever it stopped", and a run that faulted is the case that
    // was asked about — so both of these are answered here as well as on the way out of a clean run.
    report_disasm(rt, debug.disasm.as_ref());
    report_dump(rt, debug.dump.as_ref());
    report_trace(rt);
    if debug.coverage {
        report_coverage(rt);
    }
    // Which call the run was inside when it ended. A fault raised in a host call says where and not
    // who, and a report missing the caller is one line short of actionable.
    if !rt.last_call.is_empty() {
        // `Runtime::last_call` is assigned when a dispatch is *entered* — `self.last_call = label`
        // inside `Runtime::dispatch_inner` (`src/runtime/mod.rs:5491`), on
        // purpose: a fault raised inside a host call names only the address it touched, and this is
        // the one frame that knows who asked. So which call it names depends on where the run died.
        // A guest fault — a bad `ldr` of the pack's own — happens after the last dispatch returned,
        // and then this line is that innocent earlier call. Reading it as the culprit points at
        // `AEEHelperFuncs::setstaticptr`, a two-line map insert that cannot raise the error it was
        // reported for.
        println!(
            "\nlast host call entered: {} (the last dispatch this run **entered**: if the fault came \
             from inside a host call this is that call and it never returned; if it came from the \
             pack's own code this is the previous call, which did)",
            rt.last_call
        );
        // The parenthesis above is honest about not knowing which of those it is. When the answer is
        // known, say it: a refused dispatch was entered and its body never ran, so the name above is
        // the culprit *and* the address it was handed is the whole story — there is nothing inside
        // it to look at.
        if rt.last_call_refused {
            println!(
                "  ...and that is the call that was refused: it was entered and its body never ran, \
                 so the `this` it was handed is the whole story, not anything inside it"
            );
        }
    }
    // Where the run ended, said before the sentence it qualifies: "the guest reported this" and
    // "somebody ended the run while it was reporting this" are different findings, and the second one
    // is the common case in a window — a close lands between turns often enough that nothing is
    // printed at all, but a close that lands *during* a turn takes that turn's error with it. A run
    // the guest ended on its own prints nothing here, so an ordinary fault report reads as it always
    // did.
    match why {
        quoin::session::End::Cancelled => println!(
            "\nthis run was ended by request — the window was closed, Esc was pressed, or a caller \
             cancelled it — so the error below is where the machine stood when the ask arrived, not \
             necessarily a place it reached on its own"
        ),
        quoin::session::End::Quit => println!(
            "\nthis run was ended by --serve's quit, so the error below is where the machine stood \
             when the ask arrived"
        ),
        // `Finished` is the guest (or a fault) ending it, and `Running` cannot reach this function;
        // both stay silent so the ordinary crash report is unchanged.
        quoin::session::End::Finished | quoin::session::End::Running => {}
    }
    if let Some(p) = bmp {
        // The screenshot is the reason this runs on the error path at all, so a failure to write it
        // is said out loud rather than swallowed: "it faulted and here is the picture" and "it
        // faulted" are different answers.
        if let Err(e) = write_bmp(rt, p) {
            eprintln!("could not write {p}: {e}");
        }
    }
    report_threads(rt);
    // The sentence itself, and under it what it means: the report above is for someone reading the
    // code, this line is for someone who was playing a game and wants to know whose fault it is and
    // what to type next.
    let text = e.to_string();
    match quoin::hle::advice(&text) {
        Some(zh) => format!("{text}\n  note: {zh}"),
        None => text,
    }
}

/// The framebuffer as a 24-bit BMP.
fn write_bmp(rt: &Runtime, path: &str) -> Result<(), String> {
    let bytes = rt.screenshot_bmp().map_err(|e| e.to_string())?;
    std::fs::write(path, &bytes).map_err(|e| format!("{}: {}", path, e))?;
    println!(
        "{}: {}x{} RGB565 framebuffer written as {} bytes",
        path,
        rt.device.width,
        rt.device.height,
        bytes.len()
    );
    Ok(())
}

/// Every audio source the run named, as the list a decoder would start from.
///
/// No clip is decoded into samples here, so the listing is what this path can offer for
/// `ISoundPlayer`: the clips a pack
/// asked to play, followed out of the naming the vtable carries. A source that could not be read
/// is listed with its reason rather than dropped, because "the app wants a file this corpus does
/// not have" is the answer to a question somebody is about to ask.
fn report_audio(rt: &Runtime, verbose: bool) {
    // Where the host's own status callbacks went. Printed before the early return below because it is
    // a claim about the *channel*, not about the payloads: "no frame moved" only means something if
    // the code under test ran, and this is the line that says whether it did.
    let (sound, media) = rt.notified;
    if sound + media > 0 {
        println!(
            "
notified: {sound} sound, {media} media (each one a real callback into the guest)"
        );
    }
    if rt.audio.is_empty() {
        return;
    }
    let unreadable = rt.audio.iter().filter(|s| s.problem.is_some()).count();
    // The list grows with the pack's playlist — dozens of clips can cross a game's first screens — so
    // unsolicited it is one line: how many, and how many of them the corpus cannot supply. The names
    // themselves are what `--verbose` or `--dump-audio` is for.
    if !verbose {
        println!(
            "\naudio: {} source(s) named{}",
            rt.audio.len(),
            if unreadable == 0 {
                String::new()
            } else {
                format!(", {unreadable} of them unreadable — --verbose lists them")
            }
        );
        return;
    }
    println!("\naudio sources named ({}):", rt.audio.len());
    if rt.audio_skipped != 0 {
        println!(
            "  ({} of them had their bytes dropped: the audio budget was spent, so their names are \
             here and their payloads are not)",
            rt.audio_skipped
        );
    }
    for (i, s) in rt.audio.iter().enumerate() {
        let how = match s.input {
            abi::generated::SDT_FILE => "file",
            abi::generated::SDT_BUFFER => "buffer",
            _ => "other",
        };
        match &s.problem {
            Some(why) => println!("  {:3}. {how} {}: {why}", i + 1, s.name),
            None => println!(
                "  {:3}. {how} {}: {} bytes, {}",
                i + 1,
                s.name,
                s.bytes.len(),
                sniff_audio(&s.bytes)
            ),
        }
    }
}

/// What a clip's first bytes say it is. A path that does not play a clip can still name it, and the
/// name is what somebody reading the list is looking for.
///
/// The two names that mean something to this host come from the code that would have to honour them:
/// [`audio::sniff`] for the one format it synthesises and [`resource::sniff`] for the pictures a
/// pack sometimes hands the sound player instead of a score. Neither is a magic-number test done here —
/// the reason is that one decoder entry must answer for both callers. What is left is the shapes this host has
/// no authority for, and those are described as shapes, never as support.
fn sniff_audio(bytes: &[u8]) -> String {
    if let Some(mime) = quoin::audio::sniff(bytes) {
        return mime.to_string();
    }
    if let Some(mime) = quoin::resource::sniff(bytes) {
        return format!("{mime}, which is not audio");
    }
    match bytes {
        // A clip that opens like a MIDI and then is not one: `MThd` with a track count the file does
        // not carry. Said as a **shape**, never as support — the extension stays `bin`, because no
        // authority will take these bytes. The table this replaced called it `MIDI (MThd)`, and
        // dropping the hint entirely would make a half-read score look like nothing at all.
        [b'M', b'T', b'h', b'd', ..] => "MThd, but not a MIDI file this host can read".to_string(),
        [b'R', b'I', b'F', b'F', ..] => "RIFF (a waveform; this host decodes none)".to_string(),
        [] => "empty".to_string(),
        _ => "an unrecognised format".to_string(),
    }
}

/// The extension a written-out clip gets, from the same two authorities. `bin` is not a claim about
/// the bytes — it is the honest "no authority named these" answer, and the file still goes out.
fn clip_extension(bytes: &[u8]) -> &'static str {
    quoin::audio::sniff(bytes)
        .or_else(|| quoin::resource::sniff(bytes))
        .and_then(|mime| mime.rsplit_once('/').map(|(_, subtype)| subtype))
        .unwrap_or("bin")
}

/// Write every source the run named into `dir`, one file per named source, and say where each went.
///
/// The bytes, as the pack named them, under an extension only an authority is allowed to give them:
/// that is the whole of what this path offers, and the question of what to do with them can only be
/// asked once they are out. A source no authority names goes out as `.bin` — nothing about it is
/// claimed, and a tool will not be told by its name to open something it cannot.
fn write_audio(rt: &Runtime, dir: &str) -> Result<(), String> {
    let path = std::path::Path::new(dir);
    std::fs::create_dir_all(path).map_err(|e| format!("{dir}: {e}"))?;
    let mut written = 0;
    for (i, s) in rt.audio.iter().enumerate() {
        if s.bytes.is_empty() {
            continue;
        }
        // The extension comes from the same two authorities as the label above, so the file a person
        // opens and the line they read cannot name different formats. A clip neither of them names
        // goes out as `.bin`, which is the absence of a claim rather than a claim.
        let ext = clip_extension(&s.bytes);
        let file = path.join(format!("{:04}-{}.{ext}", i + 1, safe_stem(&s.name)));
        std::fs::write(&file, &s.bytes).map_err(|e| format!("{}: {e}", file.display()))?;
        written += 1;
    }
    println!(
        "{}: {} of {} audio source(s) written",
        dir,
        written,
        rt.audio.len()
    );
    Ok(())
}

/// Render every clip this host can play into `dir` as WAV, and say what it left out and why.
///
/// `--dump-audio` hands over the pack's bytes; this hands over sound. Both go by the same authority
/// ([`quoin::audio::sniff`]), so the two directories name exactly the same set of clips and a
/// person can put what they heard next to the file it was rendered from. It runs through the same
/// mixer the sound card is fed, which is the only reason the two can be trusted to agree — a headless
/// run used to have no answer at all to "what does this clip sound like", and a window plus a sound
/// card was the only way to hear it.
fn write_wav(rt: &Runtime, dir: &str, soundfont: Option<&str>) -> Result<(), String> {
    let path = std::path::Path::new(dir);
    std::fs::create_dir_all(path).map_err(|e| format!("{dir}: {e}"))?;
    let font = soundfont.map(std::path::Path::new);
    let rate = 44_100u32;
    let mut written = 0;
    let mut unplayable = 0;
    let mut cut = 0;
    let mut smaf_cut = 0;
    for (i, s) in rt.audio.iter().enumerate() {
        // A source this host has no decoder for is skipped, not written as a file of silence. The
        // sniff is the same authority the player uses: MP3/AAC/WAV are named now that Symphonia
        // decodes them, while formats with no backend remain unnamed.
        let sniffed = quoin::audio::sniff(&s.bytes);
        if s.bytes.is_empty() || sniffed.is_none() {
            unplayable += 1;
            continue;
        }
        if let Ok((source_rate, samples)) = quoin::audio::decode_audio(&s.bytes) {
            let served = samples.len() as f64 / source_rate as f64;
            // Playback streams a score, so its length is the score's own end. This offline path
            // still renders into one buffer and is bounded by `smaf::MAX_SECONDS`; a clip longer
            // than that is written short on purpose, and printing the ceiling as its length would
            // hide that.
            let length_ms = if quoin::audio::smaf::looks_like(&s.bytes) {
                quoin::audio::smaf::duration_ms(&s.bytes)
            } else {
                None
            };
            // The cap is the score's own end against the dump's ceiling, not the rendered length:
            // a render that finishes early (its voices released before the score's tail) is short
            // for a reason that has nothing to do with the ceiling.
            let capped = length_ms.is_some_and(|ms| ms > quoin::audio::smaf::MAX_SECONDS * 1_000);
            smaf_cut += capped as usize;
            let bytes = quoin::audio::pcm_wav(source_rate, &samples);
            let file = path.join(format!("{:04}-{}.wav", i + 1, safe_stem(&s.name)));
            std::fs::write(&file, &bytes).map_err(|e| format!("{}: {e}", file.display()))?;
            println!(
                "  {:04} {} -> {} ({:.1} s{}, {} bytes)",
                i + 1,
                s.name,
                file.display(),
                served,
                if capped {
                    format!(
                        " of {:.1} s, capped at the dump's {} s",
                        f64::from(length_ms.unwrap_or(0)) / 1000.0,
                        quoin::audio::smaf::MAX_SECONDS
                    )
                } else {
                    String::new()
                },
                bytes.len()
            );
            written += 1;
            continue;
        }
        if sniffed == Some(quoin::audio::MIME_MIDI) {
            // The clip's own length when this host can time it, and two seconds when it cannot: the
            // default below is one second, and the `+ 1` is the tail every clip gets, because past the
            // last event the mixer renders silence and a file that stops dead at the final note-off reads
            // as truncated. A bounded guess, then, costs bytes rather than making a wrong claim.
            let ms = quoin::audio::clip_length_ms(&s.bytes).unwrap_or(1_000) as u64;
            let want = (ms + 999) / 1_000 + 1;
            let secs = want.min(quoin::audio::WAV_SECONDS);
            cut += (secs < want) as usize;
            let (bytes, got) =
                quoin::audio::render_wav(font, &s.bytes, rate, secs as usize * rate as usize)
                    .map_err(|e| format!("{}: {e}", s.name))?;
            let file = path.join(format!("{:04}-{}.wav", i + 1, safe_stem(&s.name)));
            std::fs::write(&file, &bytes).map_err(|e| format!("{}: {e}", file.display()))?;
            println!(
                "  {:04} {} -> {} ({:.1} s, {} bytes)",
                i + 1,
                s.name,
                file.display(),
                got as f64 / rate as f64,
                bytes.len()
            );
            written += 1;
            continue;
        }
        unplayable += 1;
    }
    let mut notes = String::new();
    if cut > 0 {
        notes.push_str(&format!(", {cut} cut at {} s", quoin::audio::WAV_SECONDS));
    }
    if smaf_cut > 0 {
        notes.push_str(&format!(
            ", {smaf_cut} SMAF clip(s) stopped at {} s",
            quoin::audio::smaf::MAX_SECONDS
        ));
    }
    println!(
        "{}: {} of {} audio source(s) rendered to WAV, {} this host cannot play{}",
        dir,
        written,
        rt.audio.len(),
        unplayable,
        notes
    );
    Ok(())
}

/// A source's name, cut down to what a filesystem will take. A `SDT_BUFFER` source's name is its
/// address and a `SDT_FILE` one can carry a path, so neither is a file name as it stands.
fn safe_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let stem = stem.trim_matches('-');
    if stem.is_empty() {
        "clip".to_string()
    } else {
        stem.chars().take(48).collect()
    }
}

/// The shape of a module's expectations of us: every dispatch taken, in order.
/// The label a call into the module gets in the report. A delivery into the applet's own event
/// handler is the one downcall a person at a keyboard cares about — it is where "I pressed Z" is
/// either written down or not — so its two words are named the way the SDK names them rather than
/// left as hex a reader would have to look up. Any other callee, and any word with no name under
/// that prefix, keeps its number: the number is the finding when the name is missing.
fn downcall_label(handle_event: u32, addr: u32, args: &[u32]) -> String {
    if addr != handle_event {
        return format!("← guest {addr:#010x}");
    }
    let evt = args.get(1).copied().unwrap_or(0);
    let wp = args.get(2).copied().unwrap_or(0);
    match (abi::const_name("EVT_", evt), abi::const_name("AVK_", wp)) {
        (Some(e), Some(k)) => format!("← IApplet::HandleEvent {e} {k}"),
        (Some(e), None) => format!("← IApplet::HandleEvent {e} wp={wp:#010x}"),
        _ => format!("← IApplet::HandleEvent evt={evt:#010x} wp={wp:#010x}"),
    }
}

fn print_call_log(rt: &Runtime) {
    let calls = rt.hle.calls();
    if calls.is_empty() {
        println!("no HLE dispatches — the module never reached a host stub");
        return;
    }
    println!("\nHLE calls ({}):", rt.hle.call_total());
    println!("{}", rt.call_log());
    if !rt.log_args.is_empty() || !rt.hle.downcalls().is_empty() {
        // Folded, because a game blits the same rectangle ten thousand times: the first two
        // hundred of those are one fact, and the calls that differ are further down the list than
        // any cap that counts calls.
        const SHOW: usize = 120;
        let hits: Vec<&_> = calls
            .iter()
            .filter(|c| rt.log_args.iter().any(|pat| c.label.contains(pat.as_str())))
            .collect();
        if rt.log_args.is_empty() {
            // The rows behind this block are the only place a run says what it handed *into* the
            // pack, which is the question a player asks when the picture moves but the keys do
            // nothing. They used to ride on `--args`, so `--verbose` alone -- everything a person
            // at a window has -- could not say whether a key reached the applet at all.
            println!(
                "\nevery call into the module (the pack's own handlers: a1 is `this', a2 onward are \
                 its arguments), repeats folded:"
            );
        } else {
            println!(
                "\narguments of the {} call(s) matching {}, plus every call into the module, repeats folded (a1 is `this'; a5 onward are stack words):",
                hits.len(),
                rt.log_args.join(" or ")
            );
        }
        /// An answer below this is a number that means something — `Test` says 0 or 1, `Read` says how
        /// many bytes came back — so it belongs in the fold key. Above it the answer is a handle or a
        /// pointer, different on every call, and folding on the address would leave the report as
        /// noisy as no folding at all; those all fold together as "an object came back".
        const ANSWER_IS_A_NUMBER: u32 = 0x1000;
        let class = |retval: u32| {
            if retval < ANSWER_IS_A_NUMBER {
                format!("#{retval}")
            } else {
                "an object".to_string()
            }
        };
        struct Row {
            label: String,
            args: Vec<u32>,
            text: Vec<Option<ArgText>>,
            trail: String,
            /// What makes this row the same as another: the arguments with any pointer replaced by
            /// the name it carried, plus the answer classified. See the fold below.
            key: (Vec<String>, String),
            times: usize,
        }
        let rows = hits.iter().map(|c| {
            (
                c.label.to_string(),
                c.args.clone(),
                c.text.clone(),
                format!("-> {:#010x}", c.retval),
                class(c.retval),
            )
        });
        // A call the other way has no name to give it, only the address the module's own vtable held
        // - which is the address its answer has to be read at. `call_guest` passes the register half
        // of the AAPCS arguments, so the zeros at the end are unused registers rather than arguments.
        // The one address that DOES have a name is the applet's event handler, because the question
        // "did my key reach the pack" is answered nowhere else.
        let handle_event = rt
            .guest_slot(rt.applet, "IApplet", "HandleEvent")
            .unwrap_or(u32::MAX);
        // The folded form of the same direction, and the one that survives a long run. The history
        // below keeps its most recent half, so by the time anybody asks "did my key ever reach the
        // pack" — which is minutes into a session by definition — the pack's own per-frame callback has
        // pushed every key out of it. These counts do not expire: a run of a few minutes reports
        // thousands of callbacks and a couple of hundred events, and a history that kept only recent
        // rows would show the callbacks and not one of the events.
        let folded = rt.hle.folded_downcalls();
        if !folded.is_empty() {
            const FOLDED_SHOW: usize = 40;
            println!(
                "\nevery call into the module, folded by the object and the two words after it, \
                 counted for the whole run (not only what the history above still holds{}):",
                if rt.hle.folded_full() {
                    ", and it hit its ceiling: rows below the cut are missing"
                } else {
                    ""
                }
            );
            for ((addr, a1, a2), n) in folded.iter().take(FOLDED_SHOW) {
                println!(
                    "  {n:>8} x  {}",
                    downcall_label(handle_event, *addr, &[0, *a1, *a2, 0])
                );
            }
            if folded.len() > FOLDED_SHOW {
                println!("  … {} more folded row(s)", folded.len() - FOLDED_SHOW);
            }
        }
        let down = rt.hle.downcalls().iter().map(|d| {
            let mut args = d.args.to_vec();
            while args.last() == Some(&0) {
                args.pop();
            }
            (
                downcall_label(handle_event, d.addr, &d.args),
                args,
                Vec::new(),
                format!("-> {:#010x}", d.retval),
                class(d.retval),
            )
        });
        let mut seen: Vec<Row> = Vec::new();
        let mut dropped = 0;
        for (label, args, text, trail, answer) in rows.chain(down) {
            // Folded on the name, not the address, and not on `this` either. A pack that opens twelve
            // files copies each name into a fresh block first, and the shell hands out a new handle
            // every time, so 225 opens of a dozen distinct names are 225 distinct argument lists to a
            // key that looks at numbers — the fold becomes a no-op exactly where it is needed, and the
            // cap then drops the very calls the reader came for.
            let key: Vec<String> = args
                .iter()
                .enumerate()
                .skip(1)
                .map(|(i, a)| match text.get(i).and_then(|t| t.as_ref()) {
                    Some(t) => t.render(),
                    None => format!("{a:#010x}"),
                })
                .collect();
            if let Some(prev) = seen
                .iter_mut()
                .find(|r| r.label == label && r.key == (key.clone(), answer.clone()))
            {
                prev.times += 1;
                continue;
            }
            if seen.len() == SHOW {
                dropped += 1;
                continue;
            }
            seen.push(Row {
                label,
                args,
                text,
                trail,
                key: (key, answer),
                times: 1,
            });
        }
        for row in &seen {
            let Row {
                label,
                args,
                text,
                trail,
                times,
                ..
            } = row;
            print!("  {label}");
            for (i, a) in args.iter().enumerate() {
                print!(" {a:#010x}");
                // What the argument pointed at *when it was passed*, which is the only answer for a
                // name or a rectangle the caller built on a stack that has since been reused. A row
                // with no captured text at all — a host→guest call — still gets the late reading,
                // because a module constant has not moved and there is nothing better to show.
                let shown = if text.is_empty() {
                    rt.string_at(*a).map(ArgText::Name)
                } else {
                    text.get(i).cloned().flatten()
                };
                if let Some(t) = shown {
                    print!("{}", t.render());
                }
            }
            if *times > 1 {
                print!(" ×{times}");
            }
            println!(" {trail}");
        }
        if dropped > 0 {
            println!("  … and {dropped} more distinct argument set(s)");
        }
    }
    // Counted from `Hle::tally`, which is every dispatch the run took, and not from the `calls`
    // window above — that one drops its oldest half once it passes `Hle::HISTORY`, so a line that
    // reads as a census would silently be a window, and "not in this list" would look like "never
    // asked". A reader counts interfaces from this line, so it has to be the whole run.
    let seen = interfaces_touched(&rt.hle.tally());
    println!("interfaces touched: {seen}");
    if !rt.missing_classes.is_empty() {
        let names: Vec<String> = rt
            .missing_classes
            .iter()
            .map(|c| named(*c, "AEECLSID_"))
            .collect();
        println!("classes the host refused: {}", names.join(", "));
    }
    if !rt.missing_resources.is_empty() {
        println!(
            "resources the host could not serve: {}",
            rt.missing_resources.join(", ")
        );
    }
    if !rt.display_prefs.is_empty() {
        println!(
            "device settings the run asked IDisplay to apply: {}",
            rt.display_prefs.join(" | ")
        );
    }
}

/// What the applet actually put on the screen, which is the point of a headless run.
/// What the guest arena is charged for, and what it refused. The refusal count is the reason this exists:
/// `MALLOC()` answering NULL is the pack's own problem to check, but it is also the only way a pack
/// that has run for three hundred million instructions suddenly walks a null pointer it had used
/// all along: a run that never says a refusal happened loses the reason the next picture came back
/// empty.
fn report_heap(rt: &Runtime) {
    let mem = &rt.cpu.mem;
    let span = mem.heap_limit().saturating_sub(quoin::layout::HEAP_BASE);
    // `high` is `AEEkHeap`'s own watermark — it rises when an allocation had to `Sbrk` — and the free
    // list is what the heap has given back; the difference is what is charged, node headers
    // included. A pack that frees what it allocates keeps "in use" near zero and the watermark still.
    let reached = mem.heap_high().saturating_sub(quoin::layout::HEAP_BASE);
    let spare = mem.heap_spare();
    let used = reached.saturating_sub(spare);
    println!(
        "\nheap: {} KiB in use, {} KiB reached of the {} MiB arena, {} KiB on the free list in {} block(s)",
        used / 1024,
        reached / 1024,
        span / (1024 * 1024),
        spare / 1024,
        mem.heap_free_blocks()
    );
    if mem.alloc_fails != 0 {
        println!(
            "{}  allocation(s) refused, largest {} byte(s) — MALLOC() answered NULL, and what the \
             pack did next is its own business",
            mem.alloc_fails, mem.alloc_fail_max
        );
        // A refusal has two very different causes and the report should not leave the reader to
        // guess: the heap may simply be full, or something may have written over a node's header and
        // broken the chain the search walks. One walk answers it.
        match mem.heap_audit() {
            Ok(()) => println!(
                "  the heap's own bookkeeping still agrees with itself — the chain reaches the end \
                 and every free node is on the list once — so the refusals were about space"
            ),
            Err(why) => println!(
                "  and its bookkeeping does NOT agree with itself: a node header has been written \
                 over — {why}"
            ),
        }
    }
}

/// How far the game got, in the game's own words. A tester's question is "which screen is it on,
/// and is that screen empty", and a headless run has no picture to look at — but the strings the pack drew *are* the
/// picture as far as a report can tell: a title screen says what it says. Grouped by the screens the
/// pack put up (`IDisplay::Update`), because a pack that repaints a score every frame is still on one
/// screen.
fn report_story(rt: &Runtime) {
    // How far the session ran, and whether it ran out of work or ran out of rope. A report that says
    // only "2 presentations" cannot be told apart from a run that drew two frames and quit from one
    // that was cut off mid-animation, and that difference is the whole question being asked.
    let waiting = match (rt.timers.len(), rt.queue.len(), rt.pending_release.len()) {
        (0, 0, 0) => String::new(),
        (t, q, r) => format!(
            "; {} timer(s), {} queued job(s) and {} key(s) still waiting for their release — this run ran out of budget, it did not finish",
            t, q, r
        ),
    };
    println!(
        "\nthis run retired {} instruction(s){}",
        rt.steps(),
        waiting
    );
    let runs = &rt.display.runs;
    if runs.is_empty() {
        // "it drew not one word" is the honest half; "the screen only has graphics" was never supported by anything here,
        // because graphics reaching the screen through a host drawing call is a separate count (see
        // `drawing_calls`) and a pack can write an `IDIB` in guest memory with no host call at all.
        // What this line can say is that no text run was recorded.
        println!("where it got to: no text draw was recorded (whether it drew graphics is the drawing call(s) line above).");
        return;
    }
    let mut groups: Vec<(u64, Vec<(String, u64)>)> = Vec::new();
    if rt.display.screens_dropped > 0 {
        // The screen numbers below come out of a bounded window, so on a long run they are not the
        // pack's screen count: `Display::update` keeps `Hle::HISTORY` boundaries and drains the
        // oldest half. This says so once instead of letting "screen 3" mean something it cannot mean.
        println!(
            "  (only the most recent {} screen boundaries are kept; {} older ones were dropped. \
             \"screen N\" below is an index in that window, not the run's total; {} presentations in all)",
            rt.display.screens.len(),
            rt.display.screens_dropped,
            rt.display.screens_seen
        );
    }
    for r in runs {
        let screen = rt
            .display
            .screens
            .iter()
            .position(|&s| s >= r.step)
            .unwrap_or(rt.display.screens.len()) as u64;
        if groups.last().map(|(k, _)| *k) != Some(screen) {
            groups.push((screen, Vec::new()));
        }
        let list = &mut groups.last_mut().expect("just pushed").1;
        // A score line redrawn every frame is one entry, not sixty; only a *different* word is a new
        // thing for the reader to see.
        if list.last().map(|(t, _)| t.as_str()) != Some(r.text.as_str()) {
            list.push((r.text.clone(), r.step));
        }
    }
    const SHOW: usize = 6;
    const WORDS: usize = 6;
    let skip = groups.len().saturating_sub(SHOW);
    let dropped = rt.display.runs_seen.saturating_sub(runs.len() as u64);
    println!(
        "\nwhere it got to (the text it drew itself, {} run(s){}):",
        rt.display.runs_seen,
        if dropped == 0 {
            String::new()
        } else {
            format!("; the first {dropped} run(s) were dropped")
        }
    );
    for (gi, (_, list)) in groups[skip..].iter().enumerate() {
        let words: Vec<String> = list
            .iter()
            .take(WORDS)
            .map(|(t, _)| format!("{t:?}"))
            .collect();
        println!(
            "  screen {} (from instruction {}, {} text run(s)): {}{}",
            skip + gi + 1,
            list[0].1,
            list.len(),
            words.join(" "),
            if list.len() > WORDS { " …" } else { "" },
        );
    }
}

/// The host calls that can put pixels on the screen, counted from the trace the host already keeps.
///
/// Deliberately not called "pixels moved", and deliberately not a new counter.
/// [`Display::presentations`] has exactly one write site and counts *requests to show the screen*
/// (`IDisplay::Update` / `IGraphics::Update`), which is why a run could print 44 presentations over a
/// frame measuring one colour — the report line that reads like a picture needs a number that is about
/// drawing. This is that number: every completed call whose handler can write pixels, by method name,
/// read out of [`Hle::tally`].
///
/// Two limits stated because they are both true and both easy to forget. It counts calls that
/// returned, because the tally is written after the result, so a run that died inside its first
/// `BitBlt` reads 0 here and 0 is not evidence of a blank screen. And a pack composing into an
/// `IDIB` in guest memory moves pixels through no host call at all
/// (guest-side composition), so 0 drawn is not a verdict — it is a fact about
/// the seam, not about the framebuffer.
fn drawing_calls(rt: &Runtime) -> usize {
    // By method name, not by interface: the same operation is reached as `IDisplay::DrawRect`,
    // `IBitmap::FillRect` and `IGraphics::DrawRect`, and an `IDIB` reports under its `IBitmap` name
    // because there is no `IDIB` interface in the tables at all.
    const MOVES: &[&str] = &[
        "BitBlt",
        "StretchBlt",
        "BltIn",
        "BltOut",
        "SetPixels",
        "DrawPixel",
        "DrawHScanline",
        "FillRect",
        "ClearRect",
        "ClearViewport",
        "DrawRect",
        "DrawRoundRectangle",
        "DrawFrame",
        "Draw",
        "DrawText",
        "DrawPoint",
        "DrawLine",
        "DrawPolyline",
        "DrawPolygon",
        "DrawTriangle",
        "DrawCircle",
        "DrawArc",
        "DrawPie",
        "DrawEllipse",
        "DrawEllipticalArc",
    ];
    rt.hle
        .tally()
        .iter()
        .filter(|(label, _)| {
            label
                .rsplit_once("::")
                .is_some_and(|(_, method)| MOVES.contains(&method))
        })
        .map(|(_, n)| *n)
        .sum()
}

/// The two slices of a module's debug lines a report prints, and how many it left between them.
///
/// **Both ends**, because each half answers a different question and a report that picks one is
/// blind to the other. The tail is what a pack was saying when it stopped: a run can come to rest with
/// `r0 = 0` in front of a vtable load, and the lines a head-only window drops are the ones that place
/// the fault inside the pack's own script
/// loop rather than in an image decode. The head is the pack introducing itself: the same run's
/// `[mod] Create filemgr:0`, `My ClassID:0x…` and `ISHELL_CreateInstance Err` sit at the front, and
/// a window that shows only the tail hides them as thoroughly as the old one hid the end — which is
/// exactly what the first version of this function did.
///
/// A separate function rather than arithmetic in the print, so both halves are testable without a
/// run; the list is capped at 512 entries, and a full one is precisely the case worth getting right.
/// The split is by **position in the list**, not by content: nothing here may decide what an
/// "unimportant" line is, and a budget that shrank whenever a pack got chattier would drop the
/// start-up of a long run on a rule nobody could read off the report.
fn debug_window(lines: &[String], shown: usize) -> (&[String], &[String], usize) {
    if lines.len() <= shown {
        return (lines, &[], 0);
    }
    // A third of the budget keeps the introduction, two thirds keep the end — the end is the part
    // that cannot be recovered by rerunning with a different `--steps`.
    let head = shown / 3;
    (
        &lines[..head],
        &lines[lines.len() - (shown - head)..],
        lines.len() - shown,
    )
}

/// Say which files this run refused, and how many. Kept out of [`report_display`] because that runs
/// only on the paths holding a screen, while a refusal is not about the screen — it answers "did the
/// host have the file the pack asked for", and a run without `--start` can and does ask for one.
///
/// Above the `--verbose` gate on purpose, and in a list of its own: `Display::note` caps at
/// `Display::NOTES` and drops the oldest half when it overflows, and the runs long enough to overflow
/// it are exactly the runs where this would be the finding.
fn report_refusals(rt: &Runtime) {
    if rt.display.refused == 0 {
        return;
    }
    println!(
        "  files: {} file-level report line(s) (refusals, files that hold nothing)",
        rt.display.refused
    );
    for refused in rt.display.refusals.iter() {
        println!("    refused: {refused}");
    }
    if rt.display.refused > rt.display.refusals.len() as u64 {
        println!(
            "  (only the first {} distinct refusal(s) are named; the count above is the run)",
            rt.display.refusals.len()
        );
    }
}

/// Where the cooperative `IThread`s were left, for a pack that used any.
///
/// "idle, nothing left to run" and a guest fault both say the run stopped; neither says **who is
/// asleep**. A pack that suspended its own worker and is waiting for someone else to resume it is a
/// different case from one that simply ran out of scripted time, and the only thing that tells them
/// apart is this list plus whose registers the machine is holding at the end.
fn report_threads(rt: &Runtime) {
    if rt.threads.is_empty() {
        return;
    }
    const SHOW: usize = 12;
    let states: Vec<String> = rt
        .threads
        .iter()
        .zip(rt.thread_park_sites())
        .take(SHOW)
        .map(|(t, (_obj, pc, lr))| {
            let state = match t.state {
                ThreadState::Idle => "never started".to_string(),
                ThreadState::Running => "running".to_string(),
                ThreadState::Suspended => "suspended".to_string(),
                ThreadState::Ended(rv) => format!("ended(rv {rv})"),
            };
            format!("{:#010x} {state} at {pc:#x} lr {lr:#x}", t.obj)
        })
        .collect();
    let owner = if rt.running_thread == 0 {
        "the primordial context".to_string()
    } else {
        format!("{:#010x}", rt.running_thread)
    };
    println!(
        "\nthreads: {} of them; the machine belongs to {owner}; {}",
        rt.threads.len(),
        states.join(", ")
    );
    if rt.threads.len() > SHOW {
        println!("  (the first {SHOW} are named; the count above is the run)");
    }
}

fn report_display(rt: &Runtime, verbose: bool) {
    let drawn = drawing_calls(rt);
    println!(
        "\ndisplay: {} screen-update request(s), {} text run(s), {} drawing call(s)",
        rt.display.presentations, rt.display.runs_seen, drawn
    );
    if rt.display.presentations > 0 && drawn == 0 {
        // Said out loud because the two numbers on the line above invite exactly the wrong reading: a
        // pack can print dozens of presentations while its captured frame measures `(0,0,0)` in every
        // pixel, having only ever asked the host to show a screen it had not written.
        println!(
            "  those updates asked the host to show the screen; no host call that can draw completed \
             in this run. (0 here does not prove a blank screen: a pack can write its own `IDIB` \
             buffer with no host call to witness it.)"
        );
    }
    report_refusals(&rt);
    // How far the pack got, always: it is the first thing someone playing a game wants to know, and it
    // costs a dozen lines at most however long the run went.
    report_story(rt);
    // What the module asked the host to print for itself, and that comes out *above* the `--verbose`
    // gate below. The two lists the gate holds back are the text runs and the notes, and they are
    // gated because they grow with how long a run went; these lines do not — they are deduplicated and
    // capped — and they are the one thing a pack says about its own failure: a run can reach its own
    // `mainBREW.cpp:60` and print `INITIALIZATION FAILED!`, and a run without `--verbose` prints
    // neither: a diagnosis thrown away at the last step, and a silence that read as an answer.
    //
    // Collected rather than printed as it happened because a console write is a syscall and a pack
    // can arrive at `dbgprintf` thousands of times a frame — enough that the writing, and not the
    // emulation, was what the run spent its time on.
    if !rt.debug.is_empty() {
        println!("\nmodule debug output ({}):", rt.debug.len());
        if verbose {
            // A module's own failure line can sit anywhere in its output. The window below is a
            // reasonable default for a long run, but a person asking for `--verbose` is asking for
            // the whole sentence, including the one the window would have skipped.
            for line in &rt.debug {
                println!("  {}", line);
            }
        } else {
            const SHOWN: usize = 24;
            let (head, tail, skipped) = debug_window(&rt.debug, SHOWN);
            for line in head {
                println!("  {}", line);
            }
            if skipped != 0 {
                println!(
                    "  ({} line(s) in between are not printed; the first {} and the last {} are)",
                    skipped,
                    head.len(),
                    tail.len()
                );
            }
            for line in tail {
                println!("  {}", line);
            }
        }
        // Printed after the window because it changes what "the last" means: the list stops growing
        // once it is full, so every line spoken past that point is missing from the *end*, and the
        // tail this print keeps is the last that fitted rather than the last the module said.
        if rt.debug_dropped != 0 {
            println!(
                "  ({} more debug line(s) were said after the list filled, so they are not in it)",
                rt.debug_dropped
            );
            // The one line the cap cannot be allowed to take: a count says lines went missing, this
            // says what the pack was on when it stopped.
            if !rt.last_debug.is_empty() {
                println!(
                    "  the last thing it said, which the full list dropped: {}",
                    rt.last_debug
                );
            }
        }
    }
    // The counts above are the summary and always come out. The runs and the notes below are the
    // volume, and they are also the two lists that grow with how long a run went rather than with
    // what it did — which is what `--verbose` is asked for.
    if !verbose {
        return;
    }
    for r in rt.display.runs.iter() {
        println!(
            "  {:<16} at ({},{}) {}px wide in {} font, flags {:#010x}",
            format!("{:?}", r.text),
            r.x,
            r.y,
            r.width,
            named(r.font, "AEE_FONT_"),
            r.flags,
        );
    }
    for note in rt.display.quiet.iter() {
        println!("  note: {}", note);
    }
    if rt.display.notes_dropped != 0 {
        println!(
            "  ({} earlier note(s) were not kept)",
            rt.display.notes_dropped
        );
    }
}

/// The copy the run left behind when it stopped somewhere worth reading: a fault's copy at the trap, a
/// `--watch-stop`'s at the access it stopped on. `None` means the run neither faulted nor was asked to
/// stop, so there is no machine *to* report.
///
/// Every report that reads a stopped machine goes through this, because the three used to disagree: the
/// panel preferred the copies, `--dump r0` read the live bank (which the way out of the guest had
/// already moved), and `--disasm` with no address only knew about faults — a run stopped at a watchpoint
/// was told "nothing to list" beside its own register panel.
fn stopped_machine(rt: &Runtime) -> Option<quoin::runtime::Crash> {
    rt.crash.or(rt.watched)
}

fn stop_machine(rt: &Runtime) -> quoin::runtime::Crash {
    stopped_machine(rt).unwrap_or(quoin::runtime::Crash {
        regs: rt.cpu.r,
        pc: rt.cpu.pc,
        cpsr: rt.cpu.cpsr,
        thumb: rt.cpu.thumb(),
        steps: rt.steps(),
    })
}

/// Which of the three `stop_machine` picked, in the words the report says out loud. A register read is
/// only useful if the reader knows whose registers these are.
///
/// The middle arm has to name both instruments that produce it: `--break` and `--watch-stop` both reach
/// `Stop::Stopped` (`src/cpu/mod.rs`, and `runtime.rs` copies it into `watched`), so calling it a
/// watchpoint's copy would mislabel the commonest case — the sentence this replaces did name `--break`.
fn stop_machine_source(rt: &Runtime) -> &'static str {
    match (rt.crash, rt.watched) {
        (Some(_), _) => "the fault’s machine copy — the same registers the panel above prints",
        (None, Some(_)) => {
            "the stop’s machine copy (`--break` or `--watch-stop`) — the same registers the panel above prints"
        }
        // Not "nothing stopped this run": a cancelled run or an exhausted budget did stop, it just left
        // no copy, and these really are the registers after the last instruction executed.
        (None, None) => "the live registers after the last instruction (this run left no machine copy)",
    }
}

/// The machine as it stopped. A fault's copy was taken at the trap, because the guest may have been
/// part-way through something; a run stopped at a watchpoint — or at an unimplemented call — still has
/// its registers live. Either way this is the next thing to read, so there is one report rather than a
/// special case for the runs that did not fault.
fn report_crash(rt: &Runtime) {
    let c = stop_machine(rt);
    report_machine(rt, &c);
}

/// The same report for the two ways a run can stop with its registers worth reading: a fault, whose copy
/// was taken at the trap, and a watchpoint (`--watch-stop`), whose registers are still the guest's
/// because nothing has run since. One function, so the two cannot drift apart in what they print — the
/// whole reason to stop at an access is to read the same things a fault report reads.
fn report_machine(rt: &Runtime, c: &quoin::runtime::Crash) {
    let module = rt.module.as_ref();
    println!(
        "\nregisters after {} instruction(s), pc={} ({} state), cpsr={:#010x}:",
        c.steps,
        symbol::symbolize(&rt.hle, module, c.pc),
        if c.thumb { "Thumb" } else { "ARM" },
        c.cpsr
    );
    let words: Vec<String> = REG_NAMES
        .iter()
        .zip(c.regs)
        .map(|(name, val)| {
            let sym = symbol::symbolize(&rt.hle, module, val);
            // Only the values that have a name worth reading get one. A register holding `0x18` needs
            // no gloss, and a dump where every line carries an annotation is one where none of them is
            // seen — which is the same reason the volume is opt-in. An address inside a region with no
            // name of its own is left alone too: `<object+0x6000001>` is how a class id looks, and
            // dressing a class id up as an address is worse than saying nothing about it.
            let named = !sym.starts_with("<0x")
                && !sym.starts_with("<object+")
                && !sym.starts_with("<vtable+");
            if val != 0 && named {
                format!(" {name}={val:#010x} {sym}")
            } else {
                format!(" {name}={val:#010x}")
            }
        })
        .collect();
    // Two to a line, because some of them carry a name now and four would wrap.
    for row in words.chunks(2) {
        println!(" {}", row.concat());
    }
    // How it got here. ARM keeps no frame chain, so this is a scan rather than a walk: every word on
    // the stack that is an address inside the module is a candidate return address, newest first. It
    // is a heuristic and it is labelled as one — but "which of my functions called this one" is the
    // question a fault raises next, and a scan answers it often enough to be worth the four lines.
    //
    // The snapshot covers the *registers*; memory is not copied. That is the right split here and it is
    // structural, not a hope: every report in this file takes `&Runtime`, and the write methods on `Mem`
    // are `&mut self` (`src/mem.rs:479,634,649,742`) — the cells inside `Mem` hold watchpoint bookkeeping,
    // not page bytes — so nothing between the trap and this line can have moved a stack word.
    let frames = stack_frames(rt, c.regs[13]);
    if frames.is_empty() {
        // Said rather than skipped, because "no frames" has two very different meanings and the reader
        // cannot tell them apart from silence: the stack may hold nothing that points into the module,
        // which is what a pack that inflates its own image and runs it off the heap looks like.
        println!(
            "\nstack: no word in the first {} of it points into the module image — whatever was \
             running was not module code",
            FRAME_SCAN * 4
        );
    } else {
        println!("\nstack (candidate return addresses, newest first):");
        for (i, a) in frames.iter().enumerate() {
            println!("  {i}: {}", symbol::symbolize(&rt.hle, module, *a));
        }
    }
    // What the heap makes of the pointers in the registers. A data abort at 0 is usually one of two
    // things — a field that was never filled in, or a block the heap has already taken back and
    // handed on — and the fixes are opposite. This host keeps the heap in guest memory, so the
    // question is answerable at the fault: a node that is *free* under a register is a pointer the
    // pack released and kept, and a live one says the pack cleared or never wrote the field itself.
    let heap_lo = quoin::layout::HEAP_BASE;
    let heap_hi = rt.cpu.mem.heap_limit();
    let mut heap_words = Vec::new();
    for (i, v) in c.regs.iter().enumerate().take(13) {
        if *v < heap_lo || *v >= heap_hi {
            continue;
        }
        let said = match rt.cpu.mem.node_at(*v) {
            Some((_, true)) => "FREE node — a pointer to space the heap has taken back".to_string(),
            Some((size, false)) => format!("live node, {size:#x} bytes in all"),
            None => "not a node start in this heap".to_string(),
        };
        heap_words.push(format!(
            "  r{i} = {v:#010x} heap+{:#x}: {said}",
            v - heap_lo
        ));
    }
    if !heap_words.is_empty() {
        println!(
            "
heap: what the pointers in the registers name"
        );
        for line in heap_words {
            println!("{line}");
        }
    }
    // And whether the heap itself is in one piece. A guest that faults while walking a heap object can
    // be a guest holding a stale pointer, or a heap whose chain something has written over; the first
    // is the pack's, the second is usually the host's, and one walk tells them apart.
    if let Err(why) = rt.cpu.mem.heap_audit() {
        println!(
            "
heap: its own chain and free list disagree — a node header has been written over: {why}"
        );
    }
    // What was about to run. Printed whether or not `--disasm` was asked for: a run that faulted is
    // exactly the run whose instructions someone wants to read, and the program counter on its own is
    // not the answer to "what was it doing".
    println!("\nat {}:", symbol::symbolize(&rt.hle, module, c.pc));
    list(rt, c.pc, 6, c.thumb, Some(c.pc));
}

/// The candidate return addresses in a run of stack words: the ones that point inside the module
/// image.
///
/// This is a heuristic and it is named as one. AAPCS obliges nobody to keep a frame pointer, so there
/// is no chain to walk; what there is, is a stack full of saved `lr`s and of locals, and a word that
/// is an address into the image is almost certainly the former. The alternative — a shadow stack kept
/// by the interpreter — is a real cost on the hot path for a fact that only matters once something has
/// already gone wrong.
fn return_addresses(words: &[u32], base: u32, len: u32) -> Vec<u32> {
    words
        .iter()
        .copied()
        .filter(|w| *w >= base && *w < base.wrapping_add(len))
        .collect()
}

/// How far up the stack to look. A run's own frames are near the top of it; past a certain point the
/// words are the applet's data and the answer turns into noise.
const FRAME_SCAN: usize = 192;

/// The stack frames a fault can see, newest first.
/// The architectural register names, in the order the dumps print them: index 13 is `sp` and 14 is `lr`.
/// Both the register dump and `--break ADDR,r4=0` read this list, so the two cannot disagree about
/// which number a name means.
const REG_NAMES: [&str; 15] = [
    "r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10", "r11", "r12", "sp", "lr",
];

fn register_number(name: &str) -> Option<u32> {
    REG_NAMES
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name.trim()))
        .map(|i| i as u32)
}

/// The stack frames a fault can see, newest first.
fn stack_frames(rt: &Runtime, sp: u32) -> Vec<u32> {
    let Some(loaded) = rt.module.as_ref() else {
        return Vec::new();
    };
    let mut words = Vec::new();
    for i in 0..FRAME_SCAN {
        match rt.cpu.mem.read_u32(sp.wrapping_add(i as u32 * 4)) {
            Ok(w) => words.push(w),
            // An unmapped word ends the scan rather than stepping over it: the stack that matters is
            // the mapped part, and whatever lies past the fault is not this frame's business.
            Err(_) => break,
        }
    }
    return_addresses(&words, loaded.base, loaded.len)
}

/// A listing of `count` instructions at `addr`, in ARM or Thumb state, with `mark` pointed out when
/// it is one of the lines. Reading can fail — the address may be unmapped, which is often the whole
/// finding — so that comes back as the listing's answer rather than as this function's error.
fn list(rt: &Runtime, addr: u32, count: usize, thumb: bool, mark: Option<u32>) {
    let module = rt.module.as_ref();
    let span = count * 4;
    let bytes = match rt.cpu.mem.read_bytes(addr, span) {
        Ok(bytes) => bytes,
        Err(e) => {
            println!(
                "  cannot read {span} bytes at {}: {e}",
                symbol::symbolize(&rt.hle, module, addr)
            );
            return;
        }
    };
    for line in disasm::disassemble(&bytes, addr, thumb, count) {
        let here = if Some(line.addr) == mark { "→" } else { " " };
        println!("{here} {}", line.render());
    }
}

/// Where `--disasm` lists, decided apart from printing it.
///
/// The precedence is a value rather than a `println!` because that is exactly what went wrong here: two
/// reports read different things for the same question, and a function that only prints cannot be
/// asserted on. `mark` is the pc the arrow sits on, when the listing has one of the run's own.
#[derive(Debug)]
enum Listing {
    At {
        at: u32,
        thumb: bool,
        mark: Option<u32>,
    },
    /// The caller named something this host cannot resolve, and has to be told so with its own words back.
    Unparseable(String),
    /// No address named and no stop to list: said out loud rather than left as silence.
    Nothing,
}

/// What `--disasm` asked for, once the run has stopped. With no address given the listing is of where
/// the run ended: a fault's program counter or a `--watch-stop`'s — either copy [`stopped_machine`]
/// hands over.
fn disasm_listing(rt: &Runtime, addr: Option<&String>) -> Listing {
    let stop = stopped_machine(rt);
    match (addr, stop) {
        (Some(text), _) => {
            let Some(at) = symbol::parse(rt.module.as_ref(), text) else {
                return Listing::Unparseable(text.clone());
            };
            // An odd address means Thumb state, which is the same bit every `bx`/`blx` target carries.
            // Without it a Thumb address cannot be named here at all: a run that stopped inside a downcall
            // has no `crash.thumb` to fall back on, and a pack's own code is reached exactly that way.
            // Where the run *did* stop somewhere, its own record of the state wins over the parity rule
            // for that one address — an even Thumb halfword is otherwise indistinguishable from an ARM
            // word, and listing a fault's own instruction in the wrong state walks the decoder into
            // bytes it cannot name.
            let thumb = match stop {
                Some(c) if (c.pc & !1) == (at & !1) => c.thumb,
                _ => at & 1 == 1,
            };
            Listing::At {
                at: at & !1,
                thumb,
                mark: stop.map(|c| c.pc),
            }
        }
        (None, Some(c)) => Listing::At {
            at: c.pc,
            thumb: c.thumb,
            mark: Some(c.pc),
        },
        (None, None) => Listing::Nothing,
    }
}

fn report_disasm(rt: &Runtime, req: Option<&(Option<String>, usize)>) {
    let Some((addr, count)) = req else { return };
    let module = rt.module.as_ref();
    let (at, thumb, mark) = match disasm_listing(rt, addr.as_ref()) {
        Listing::At { at, thumb, mark } => (at, thumb, mark),
        Listing::Unparseable(text) => {
            println!(
                "\n--disasm {text:?}: not an address this host knows — try 0x…, module+0x…, \
                 heap+0x…, stack-0x… or IShell::CreateInstance"
            );
            return;
        }
        Listing::Nothing => {
            println!(
                "\n--disasm: nothing to list — the run neither faulted nor stopped on purpose, \
                 and no address was named"
            );
            return;
        }
    };
    println!(
        "\ndisassembly at {}:",
        symbol::symbolize(&rt.hle, module, at)
    );
    // The arrow sits on the stopped instruction, whichever stop it was: a listing whose marker is one
    // instruction past its subject is how a fault report once pointed at the fetch after the load.
    list(rt, at, *count, thumb, mark);
}

fn dump_address(rt: &Runtime, text: &str) -> Option<(u32, Option<&'static str>)> {
    if let Some(n) = register_number(text) {
        let name = REG_NAMES.get(n as usize).copied()?;
        return Some((stop_machine(rt).regs[n as usize], Some(name)));
    }
    symbol::parse(rt.module.as_ref(), text).map(|at| (at, None))
}

/// The part of a `--dump` line that says *what was pointed at*: the symbol, and in front of it the
/// register the address came from when it came from one.
///
/// Both the read and its failure start with this, so the name cannot appear on one line and drop out of
/// the other — which is the defect this replaces. The two lines are the same answer to the same
/// question, and the question is whose pointer this is.
fn dump_header(rt: &Runtime, at: u32, from: Option<&str>) -> String {
    format!(
        "{}{}",
        from.map(|name| format!("{name} = ")).unwrap_or_default(),
        symbol::symbolize(&rt.hle, rt.module.as_ref(), at)
    )
}

/// The line that says *whose* register a `--dump` read, or `None` when the text named a place and no
/// register is in play. Its own function because both lines that can carry it print it — the dump and
/// the refusal — and the refusal is the one that once misled: `--dump r0 = NULL: cannot read 64
/// byte(s)` reads as "this register held nothing" when the truth can be "you are looking at the
/// machine that was left after the way out".
fn dump_machine_line(rt: &Runtime, from: Option<&'static str>) -> Option<String> {
    from.map(|name| format!("  ({name} read from {})", stop_machine_source(rt)))
}

/// What `--dump` asked for: the bytes at an address and — the half a hex dump cannot give you — which
/// of the words inside them are names this host knows.
///
/// Reading can fail, and that is itself an answer: an address the guest holds that this host never
/// mapped is usually the whole finding.
fn report_dump(rt: &Runtime, req: Option<&(String, usize)>) {
    let Some((text, len)) = req else { return };
    let module = rt.module.as_ref();
    let Some((at, from)) = dump_address(rt, text) else {
        println!(
            "\n--dump {text:?}: not an address this host knows — try 0x…, module+0x…, heap+0x…, \
             stack-0x…, a register (r0…r12, sp, lr) or IShell::CreateInstance"
        );
        return;
    };
    let machine = dump_machine_line(rt, from);
    let bytes = match rt.cpu.mem.read_bytes(at, *len) {
        Ok(bytes) => bytes,
        Err(e) => {
            // Named by the register too. "cannot read address 0" cannot be told apart from the wrong
            // register having been named, and whether the object lives in another register is the
            // question a reader actually needs answered.
            println!(
                "\n--dump {}: cannot read {len} byte(s): {e}",
                dump_header(rt, at, from)
            );
            // And which machine it came from, on this path above all: this is the line that once read
            // as "the register held nothing" while the registers had already been moved on the way out.
            if let Some(line) = &machine {
                println!("{line}");
            }
            return;
        }
    };
    println!(
        "\ndump {} ({} byte(s)):",
        dump_header(rt, at, from),
        bytes.len()
    );
    if let Some(line) = &machine {
        println!("{line}");
    }
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let base = at.wrapping_add((i * 16) as u32);
        let hex: String = chunk.iter().map(|b| format!("{b:02x} ")).collect();
        let text: String = chunk
            .iter()
            .map(|b| {
                if b.is_ascii_graphic() || *b == b' ' {
                    *b as char
                } else {
                    '.'
                }
            })
            .collect();
        println!("  {base:#010x}  {hex:<48} |{text}|");
    }
    let mut named_words = Vec::new();
    for (i, w) in bytes.chunks_exact(4).enumerate() {
        let word = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
        let sym = symbol::symbolize(&rt.hle, module, word);
        let named =
            !sym.starts_with("<0x") && !sym.starts_with("<object+") && !sym.starts_with("<vtable+");
        if word != 0 && named {
            named_words.push(format!(
                "    {:#010x}  {word:#010x}  {sym}",
                at.wrapping_add((i * 4) as u32)
            ));
        }
    }
    if named_words.is_empty() {
        println!("  (no word in it names anything: no pointers, no stub addresses)");
    } else {
        println!("  words that name something:");
        for w in named_words {
            println!("{w}");
        }
    }
    // And the bytes as text, when that is what they are: a name read out of guest memory is worth
    // seeing as one, and a run that is looking at the wrong address is usually looking at one.
    if let Ok(s) = rt.cpu.mem.read_cstr(at, *len) {
        if !s.is_empty() && s.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
            println!("  as a string: {:?}", String::from_utf8_lossy(&s));
        }
    }
}

/// Whether a `--trace-limit` stopped the trace before the run stopped.
///
/// Said in the report rather than at the moment it happens: the trace itself goes to stderr as it
/// arrives, and a line interleaved with tens of thousands of them is the one line nobody finds. A
/// truncated trace that does not say so is worse than no trace, because the reader's next conclusion is
/// about the program rather than about the log.
fn report_trace(rt: &Runtime) {
    let trace = rt.cpu.trace;
    if !trace.on() || trace.printed < trace.limit {
        return;
    }
    println!(
        "\ntrace stopped after {} instruction(s) — the run went on, the printing did not: raise \
         --trace-limit, or narrow --trace-range",
        trace.printed
    );
}

/// What this run asked the host for, folded so that the next thing to build is the first thing read.
///
/// It is deliberately dynamic rather than a checked-in list of what the host implements: such a list is
/// a second copy of `runtime.rs`'s match arms, and the copy is the half that goes stale. What cannot go
/// stale is the record of what the guest reached — which is the question anyway when a pack stops at a
/// gap.
/// What a directory run ends with: how each pack's own art lines up against the size somebody typed
/// into a folder name, and which packs the disagreements are in. Every pack printed its full report
/// above; this prints only what is worth acting on, because reading thirty-five screens of detail is
/// not the point of asking for them.
fn report_probe(reports: &[quoin::probe::Probe]) {
    let named = reports.iter().filter(|p| p.screen.is_some()).count();
    println!("--- summary: {} pack(s) ---", reports.len());
    println!(
        "  art answered the screen for {named}; it could not for {} (of which {} did not even name one in the directory)",
        reports.len() - named,
        reports
            .iter()
            .filter(|p| p.screen.is_none() && p.hint.is_none())
            .count()
    );
    let both: Vec<&quoin::probe::Probe> = reports.iter().filter(|p| p.agrees().is_some()).collect();
    let agree = both.iter().filter(|p| p.agrees() == Some(true)).count();
    let thin = reports.iter().filter(|p| p.caveat().is_some()).count();
    println!(
        "  of {} pack(s) with both witnesses, {agree} agree and {} disagree",
        both.len(),
        both.len() - agree
    );
    println!(
        "  {thin} pack(s) marked \"evidence too thin\": the containers this host can read carry no art \
         for them, so those widths describe an icon and must not drive `--device`"
    );
    for p in both.iter().filter(|p| p.agrees() != Some(true)) {
        println!("  disagreements: {}", p.path.display());
        println!("    {}", p.verdict());
    }
}

fn report_coverage(rt: &Runtime) {
    let folded = coverage(&rt.hle.tally());
    println!("\ncoverage: what this run asked the host for");
    if folded.is_empty() {
        println!("  nothing — the module never reached a host stub");
    }
    for (iface, methods, total) in &folded {
        let reached = match declared_methods(iface) {
            Some(all) => format!("{} of {all} reached", methods.len()),
            None => format!("{} reached", methods.len()),
        };
        println!("\n  {iface} — {reached}, {total} call(s)");
        for (method, n) in methods {
            println!("    {method:<26} {n}");
        }
    }
    if !rt.missing_classes.is_empty() {
        println!("\n  classes nobody served ({}):", rt.missing_classes.len());
        for cls in &rt.missing_classes {
            match abi::clsid_name(*cls) {
                Some(name) => println!("    {cls:#010x}  {name}"),
                None => println!("    {cls:#010x}  (not a class id the SDK publishes)"),
            }
        }
    }
    // The other half of the same question, and the half a run that died on a branch to address zero
    // needs. A class the host cannot serve is forwarded to the pack's own module, and a shipped
    // `AEEClsCreateInstance` is free to compare nothing and say yes to anything — so an id below is
    // one the *module* claimed, which nothing in the block above can show, and the vtable word count
    // is why that matters: a three-word table answering a class whose interface publishes eight is the
    // crash this run has not got to yet.
    //
    // That last clause is a verdict, though, and only one of the two groups beneath it may carry it.
    // An id the SDK publishes no interface for has nothing its object's word count can be measured
    // against, and that is the ordinary case — every module's own applet class lands there, working
    // ones included. Listing those beside the accused makes a heading that is half noise, and a reader
    // who has learned to skip that half skips the half that is signal. So the list splits in two, and
    // each heading counts over its own group: the number beside a heading is the number a reader acts
    // on.
    if !rt.accepted_classes.is_empty() {
        let judged = rt.accepted_short_classes();
        let unjudged = rt.accepted_unjudged_classes();
        if !judged.is_empty() {
            println!(
                "\n  classes the module accepted but may not implement ({}):",
                judged.len()
            );
            for a in &judged {
                println!("{}", accepted_class_line(a));
                let (iface, want) = a.published.unwrap();
                let have = a.vtable_words.unwrap();
                println!(
                    "      this is the one to read: {iface} publishes {want} slots and this object has \
                     {have}, so slots {have}..{} are read past its end and branch to whatever follows",
                    want - 1
                );
            }
        }
        if !unjudged.is_empty() {
            println!(
                "\n  classes the module accepted that we cannot judge ({}):",
                unjudged.len()
            );
            println!(
                "  (no interface published for the id to count its object's table against, or a table \
                 with room enough for the one it names — a word count here is a measurement, not a \
                 verdict, and a module's own applet class is always in this half)"
            );
            for a in &unjudged {
                println!("{}", accepted_class_line(a));
            }
        }
        if rt.accepted_classes_dropped != 0 {
            println!(
                "  ({} more were not kept, because a list this long hides the line that matters — and \
                 a module that takes this many takes everything)",
                rt.accepted_classes_dropped
            );
        }
    }
    if !rt.missing_resources.is_empty() {
        println!(
            "\n  resources it could not serve ({}):",
            rt.missing_resources.len()
        );
        for r in &rt.missing_resources {
            println!("    {r}");
        }
    }
    // And what the heap's own bookkeeping says at the end of the run. It belongs beside the classes
    // because it is the same kind of statement — what this run's *host* did — and because a chain
    // that no longer agrees with itself sends every later `malloc` down a walk that means nothing,
    // which a report that only counts calls would leave invisible.
    let mem = &rt.cpu.mem;
    let reached = mem.heap_high().saturating_sub(quoin::layout::HEAP_BASE);
    match mem.heap_audit() {
        Ok(()) => println!(
            "\n  heap: the chain and the free list agree — {} KiB reached, {} KiB free in {} \
             block(s)",
            reached / 1024,
            mem.heap_spare() / 1024,
            mem.heap_free_blocks()
        ),
        Err(why) => println!(
            "\n  heap: the chain and the free list do NOT agree — every allocation after it broke \
             walked a heap that means something else: {why}"
        ),
    }
}

/// One accepted class as its report line: the id, the SDK's name for it where it publishes one, what
/// the module answered, the object it published and how long that object's vtable reads. The two
/// groups in [`report_coverage`] print the same words — what separates them is the heading they sit
/// under, and the verdict that goes with it, and nothing here.
fn accepted_class_line(a: &AcceptedClass) -> String {
    let answered = match (&a.fault, a.rv) {
        (Some(why), _) => why.clone(),
        (None, Some(rv)) => result_name(rv),
        (None, None) => "no answer".to_string(),
    };
    let object = match (a.object, a.vtable_words) {
        (0, _) => "no object".to_string(),
        (obj, Some(words)) => format!("object {obj:#010x}, vtable {words} word(s)"),
        (obj, None) => format!("object {obj:#010x}, vtable length not readable"),
    };
    let asked = if a.times > 1 {
        format!("  ({} times)", a.times)
    } else {
        String::new()
    };
    format!(
        "    {:#010x}  {}  -> {answered}, {object}{asked}",
        a.cls,
        a.name.unwrap_or("(not a class id the SDK publishes)"),
    )
}

/// The dispatch record folded into *(interface, its methods by call count, its total)*, busiest
/// interface first. The input is the run's own tally rather than the call history, because the history
/// is capped and this is the one report whose numbers someone adds up. This is the part with a test of
/// its own: the fold is the answer to "what does this pack want", and that should not have to be read
/// out of a print statement.
fn coverage(tally: &[(String, usize)]) -> Vec<(String, Vec<(String, usize)>, usize)> {
    use std::collections::BTreeMap;
    let mut by: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for (label, n) in tally {
        // A label is `Iface::Method` for a vtable slot and for a helper field alike, which is what lets
        // one fold cover both. Anything without that shape keeps a heading of its own rather than being
        // dropped from a report about everything the run reached.
        let (iface, method) = match label.split_once("::") {
            Some((i, m)) => (i.to_string(), m.to_string()),
            None => ("(unlabelled)".to_string(), label.clone()),
        };
        *by.entry(iface).or_default().entry(method).or_default() += n;
    }
    let mut out: Vec<(String, Vec<(String, usize)>, usize)> = by
        .into_iter()
        .map(|(iface, methods)| {
            let mut methods: Vec<(String, usize)> = methods.into_iter().collect();
            methods.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let total = methods.iter().map(|(_, n)| *n).sum();
            (iface, methods, total)
        })
        .collect();
    out.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    out
}

/// The `interfaces touched` line: one name per interface with how many dispatches reached it,
/// busiest first. It takes the tally rather than the history on purpose — the same fold `--coverage`
/// prints, so the two cannot disagree about what a run asked for.
fn interfaces_touched(tally: &[(String, usize)]) -> String {
    coverage(tally)
        .into_iter()
        .map(|(iface, _, total)| format!("{iface}×{total}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// How many members the ABI tables declare for a name — the denominator of "reached so far". `None`
/// for a name they do not have, which is not a failure: a label the fold invented is still reported,
/// only without a fraction.
fn declared_methods(iface: &str) -> Option<usize> {
    if iface == abi::HELPER_STRUCT {
        return abi::struct_info(abi::HELPER_STRUCT).map(|s| s.fields.len());
    }
    abi::iface(iface).map(|i| i.methods.len())
}

/// `--sample`: where the guest was, at the end of the run.
///
/// Two views because they answer two different questions. The histogram says what the pack was *busy
/// with* — a hang inside one function comes out as one address holding most of the window. The tail in
/// time order says what it was *doing last*, which is what separates a pack that stopped moving from
/// one that was still working when the window closed. Both print whether the run ended by itself, on a
/// fault, or because a person closed it, because the question this exists for — "where did it stop?" — gets
/// asked in all three.
fn report_samples(rt: &Runtime) {
    let s = &rt.cpu.samples;
    if s.every == 0 {
        return;
    }
    if s.ring.is_empty() {
        // Armed and nothing recorded: the run retired fewer instructions than one interval, so the
        // honest answer is that the window never opened, not an empty list a reader must interpret.
        println!(
            "\n--sample: every {} instruction(s), {} retired — too few to take one sample",
            s.every,
            rt.steps()
        );
        return;
    }
    let first = *s.ring.front().unwrap();
    let last = *s.ring.back().unwrap();
    let module = rt.module.as_ref();
    println!(
        "\n--sample: {} sample(s) at every {} instruction(s); this ring covers {}..{} of the {} this \
         run retired, oldest first{}",
        s.ring.len(),
        s.every,
        first.at,
        last.at,
        rt.steps(),
        if first.at > 0 {
            " (earlier samples were dropped from the front)"
        } else {
            ""
        }
    );
    let hot = s.hottest();
    println!(
        "  busiest in that window ({} distinct address(es)):",
        hot.len()
    );
    for ((pc, thumb), n) in hot.iter().take(12) {
        println!(
            "  {:>6} x  {} ({})",
            n,
            symbol::symbolize(&rt.hle, module, *pc),
            if *thumb { "Thumb" } else { "ARM" }
        );
    }
    if hot.len() > 12 {
        println!("  … {} more not shown", hot.len() - 12);
    }
    println!("  the last {} of them, oldest first:", s.ring.len().min(20));
    let tail: Vec<_> = s.ring.iter().rev().take(20).cloned().collect();
    for x in tail.into_iter().rev() {
        println!(
            "    {}  {} ({})",
            x.at,
            symbol::symbolize(&rt.hle, module, x.pc),
            if x.thumb { "Thumb" } else { "ARM" }
        );
    }
}

/// What the run spent itself on, as a tree: `--flame`.
///
/// The report is built by `quoin::profile` rather than here so that the window's developer page and
/// this both read the same rows, and so that the text has a witness that does not need a pack
/// (`src/test/profile.rs` asserts a report over a tree it built by hand).
///
/// Silent when nothing was folded, which is a different statement from "nothing happened": an
/// unarmed run, a load that stopped before `AEEMod_Load` entered the module, and a run that really
/// reached no host call all look alike from here, so the line says which of them it might be rather
/// than printing an empty tree.
fn report_flame(rt: &Runtime, wanted: bool) {
    if !wanted {
        return;
    }
    let tree = rt.profile.tree();
    if tree.is_empty() {
        println!(
            "\n--flame: nothing folded — the run reached no host call, or the load stopped before \
             `AEEMod_Load` entered the module"
        );
        return;
    }
    let handle_event = rt
        .guest_slot(rt.applet, "IApplet", "HandleEvent")
        .unwrap_or(u32::MAX);
    println!(
        "\n{}",
        tree.report(handle_event, quoin::profile::Options::default())
    );
}

/// What each `--break` asked for and how many times the guest actually arrived there.
///
/// The reason this exists is the false zero: a run that ends on a fault or its time limit rather than
/// on a breakpoint reads the same from its last line as one that stopped where it was told to. So the
/// tally is printed whether or not the breakpoint fired, and zero arrivals is an answer — the address
/// was never reached — rather than a missing one.
fn report_break(rt: &Runtime) {
    if rt.cpu.break_at.is_empty() {
        return;
    }
    let module = rt.module.as_ref();
    println!("\n--break:");
    for b in &rt.cpu.break_at {
        let awaited = match b.when {
            None => format!("arrival #{}", b.after.max(1)),
            Some(t) => match REG_NAMES.get(t.reg as usize).copied() {
                Some(name) => format!("{name} == {:#010x}", t.val),
                None => "?".to_string(),
            },
        };
        println!(
            "  {} waited for {awaited}: {} arrival(s)",
            symbol::symbolize(&rt.hle, module, b.addr),
            b.seen
        );
    }
}

/// What `--watch` collected. Consecutive watched bytes are folded back into the store they came
/// from, because one `str` of a pointer is four writes and reporting four lines is a reliable way to
/// miss the one fact that matters: which value replaced which, and who did it.
fn report_watch(rt: &Runtime) {
    let spans = rt.cpu.mem.watch.clone();
    if spans.is_empty() {
        return;
    }
    let all = rt.cpu.mem.watch_hits.borrow();
    let all_arena = rt.cpu.mem.watch_arena.borrow();
    if all.len() == rt.cpu.mem.watch_limit() {
        println!(
            "  (only the first {} accesses are kept, so the middle of the run is missing)",
            rt.cpu.mem.watch_limit()
        );
    }
    if rt.cpu.mem.watch_arena_capped() {
        println!(
            "  (the arena record holds its first {} events, so anything after that is missing \
             from it — a span below may read as untouched when it was handed back later)",
            rt.cpu.mem.watch_limit()
        );
    }
    // One section per armed address, including the spans that saw nothing. With several addresses
    // watched at once the silent ones answer as often as the busy ones: "the canvas was written and
    // the staging buffer never was" is the difference between two hypotheses, and a report that
    // prints only the spans with traffic cannot show it.
    for span in spans {
        let (lo, hi) = span;
        let hits: Vec<quoin::mem::WatchHit> =
            all.iter().filter(|h| h.span == span).cloned().collect();
        let watched = if hi == lo + 4 {
            format!("{lo:#010x}")
        } else {
            format!("{lo:#010x}..{hi:#010x}")
        };
        let reads = hits.iter().filter(|h| h.read).count();
        println!(
            "\nwatch {watched}: {} byte write(s), {} byte read(s)",
            hits.len() - reads,
            reads
        );
        if hits.is_empty() {
            println!("  nothing ever touched it");
        }
        // The arena's side of the same span, printed whether or not any byte access was caught:
        // `alloc` and `free` move no bytes, so a span that reports no traffic is still answered here
        // — "this space was handed out fresh / was recycled / nobody handed it back" is a real result,
        // and it is the one that says which party put the block back into circulation.
        let arena: Vec<&quoin::mem::WatchArena> = all_arena
            .iter()
            // `len.max(1)` so a `"free-refused"`, which has no length to claim, still shows when its
            // address is inside the span.
            .filter(|e| e.start < hi && e.start.saturating_add(e.len.max(1)) > lo)
            .collect();
        if arena.is_empty() {
            println!(
                "  arena: no allocation or free landed across this span (in what the record kept)"
            );
        } else {
            println!("  arena: this space changed hands {} time(s)", arena.len());
            for e in arena {
                let at = match e.steps {
                    Some(s) => format!("{s}"),
                    // Not "the watch was not armed" — it was, or this record would not exist. The run
                    // had simply not retired an instruction yet, which is the loader's own state.
                    None => "no instruction retired".to_string(),
                };
                let origin = match e.event {
                    "free" => String::new(),
                    // A refusal has no size and no owner to report; saying so beats printing an
                    // allocator's wording next to an address the arena never handed out.
                    "free-refused" => "  not space this arena owned".to_string(),
                    _ => match e.returned_at {
                        Some(then) => {
                            format!("  recycled: this address was given back at instruction {then}")
                        }
                        None => "  no earlier free names this address".to_string(),
                    },
                };
                let call = match &e.call {
                    Some(c) => format!("  {} = {}", c.label, c.words()),
                    None => "  no host call in flight".to_string(),
                };
                let len = e.len;
                println!(
                    "    {at:>13}  {:<5} {:#010x} len {len:#x} ({len}){origin}{call}",
                    e.event, e.start
                );
                // The heap's own verdict at this handover. A chain that is already broken here dates
                // the damage to this event or an earlier one, which is the question a watchpoint on
                // a block answers next.
                if let Some(why) = &e.chain {
                    println!("      and the heap's chain no longer agrees with itself: {why}");
                }
            }
        }
        if hits.is_empty() {
            continue;
        }
        const SHOW: usize = 24;
        let mut lines: Vec<String> = Vec::new();
        let mut i = 0;
        while i < hits.len() {
            let mut j = i + 1;
            while j < hits.len()
                && hits[j].by == hits[j - 1].by
                // A folded run prints the first byte's attribution for all of it, so two accesses from
                // different host calls must stay on separate lines even when they land side by side —
                // and "different call" means the words too, not just the method: two `Read`s of four
                // bytes each, one after the other, are two answers to the question being asked.
                && hits[j].call == hits[j - 1].call
                && hits[j].addr == hits[j - 1].addr + 1
            {
                j += 1;
            }
            let run = &hits[i..j];
            // The value is assembled little-endian into a word, so a store wider than four bytes - a
            // memcpy, say - is reported by the first word it touched.
            let width = run.len().min(4);
            let (mut old, mut new) = (0u32, 0u32);
            for (k, h) in run.iter().take(width).enumerate() {
                old |= u32::from(h.old) << (8 * k);
                new |= u32::from(h.new) << (8 * k);
            }
            let mask = if width == 4 {
                u32::MAX
            } else {
                (1u32 << (8 * width)) - 1
            };
            let who = match run[0].by {
                quoin::mem::WatchBy::Guest { pc, insn } => {
                    format!("pc={pc:#010x} insn={insn:#010x}")
                }
                quoin::mem::WatchBy::Helper { pc, insn } => {
                    format!("helper of pc={pc:#010x} insn={insn:#010x}")
                }
                quoin::mem::WatchBy::Host => "host".to_string(),
            };
            // The method and the words it was handed, whenever a host call did the access. This is the
            // half the call log cannot give: that window drops the oldest entries, and the dispatch
            // worth reading is the last one before the fault.
            let via = match &run[0].call {
                Some(c) => format!("  {} = {}", c.label, c.words()),
                None => String::new(),
            };
            // A read is folded the same way a store is, so one `ldr` of a watched word is one line
            // rather than four — and the direction is spelled out, because "who read this" and "who
            // wrote this" are the two different questions a watchpoint is asked.
            let how = if run[0].read { "read " } else { "write" };
            lines.push(format!(
                "{how} {:#010x} {:>width$} -> {:>width$}  {who}{via}",
                run[0].addr,
                format!("{:#x}", old & mask),
                format!("{:#x}", new & mask),
                width = width * 2 + 2,
            ));
            i = j;
        }
        // Both ends, because "what last touched this byte" is as common a question as "what touched it
        // first", and a record that prints only its beginning answers exactly one of them. A long run's
        // answer is usually at the end: the boot-time traffic that fills the head of the record is the
        // least interesting thing in it.
        if lines.len() > SHOW {
            let head = SHOW / 2;
            let tail = SHOW - head;
            for line in &lines[..head] {
                println!("  {line}");
            }
            println!(
                "  … {} access(es) between the two halves …",
                lines.len() - SHOW
            );
            for line in &lines[lines.len() - tail..] {
                println!("  {line}");
            }
        } else {
            for line in &lines {
                println!("  {line}");
            }
        }
    }
}

/// `--event E[,WP[,DWP]]`, with each field resolved the same way a class id is.
fn parse_event(v: &str) -> Result<Cmd, String> {
    let mut fields = v.split(',');
    let evt = resolve(fields.next().ok_or("--event needs an event")?)?;
    let wp = match fields.next() {
        None => 0,
        Some(s) => resolve(s)?,
    };
    let dwp = match fields.next() {
        None => 0,
        Some(s) => resolve(s)?,
    };
    if fields.next().is_some() {
        return Err(format!("--event takes at most three fields: {v}"));
    }
    Ok(Cmd::Event { evt, wp, dwp })
}

/// A screen coordinate. Negative and hex are both allowed, because the handset's own values are
/// signed 32-bit — the `AEEEvent.h` example carries `y=FFFFFFFB` for minus five — and a pack that
/// drags a scene can be told to start one off the edge.
fn parse_coord(v: &str) -> Result<i32, String> {
    let v = v.trim();
    let bad = || format!("not a number: {v}");
    let (neg, body) = match v.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, v),
    };
    let n = match body.strip_prefix("0x").or(body.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).map_err(|_| bad())? as i64,
        None => body.parse::<i64>().map_err(|_| bad())?,
    };
    let n = if neg { -n } else { n };
    i32::try_from(n).map_err(|_| format!("out of range for a coordinate: {v}"))
}

/// Turn the TSV written by `--record-input` into the same queued key edges a live window produces.
///
/// Every gap between two recorded edges becomes a wait, so the replay carries the session's own
/// virtual timing rather than a fixed `--hold`. Pointer records use a different column shape and are
/// skipped; `--help` says so rather than dropping them silently.
///
/// The recorded file starts when the recorder was created, not when the first key went down, so a
/// key already held at that moment appears only as a `key up`. The replay keeps the observed edges
/// and invents nothing: a press that happened before the recorder existed is not ours to add, and
/// synthesising one would hand the pack a long hold it never made.
fn input_log_commands(path: &str) -> Result<Vec<Cmd>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut events: Vec<(u32, bool, u32)> = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 4 {
            return Err(format!(
                "{path}:{}: expected virtual_ms, wall_ms, event, key",
                line_no + 1
            ));
        }
        let at: u32 = fields[0].parse().map_err(|_| {
            format!(
                "{path}:{}: not a virtual millisecond: {}",
                line_no + 1,
                fields[0]
            )
        })?;
        let event = fields[2];
        let code = match event {
            "key down" | "key up" => resolve(fields[3])?,
            // Pointer records use a different column shape; this replay is for the key spine that
            // reproduces a map transition. Dropping them is stated in `--help`, not silent.
            "pointer" => continue,
            other => {
                return Err(format!(
                    "{path}:{}: unknown input event {other}",
                    line_no + 1
                ))
            }
        };
        events.push((at, event == "key down", code));
    }
    if events.is_empty() {
        return Err(format!("{path}: no key events"));
    }

    let mut out = Vec::new();
    let mut last = 0;
    for (at, down, code) in events {
        if at < last {
            return Err(format!("{path}: event times go backwards at {at} ms"));
        }
        if at > last {
            out.push(Cmd::Wait(at - last));
            last = at;
        }
        out.push(if down {
            Cmd::KeyDown(code)
        } else {
            Cmd::KeyUp(code)
        });
    }
    // The point of replaying a stuck session is the time after its last input. Five seconds is
    // enough to see whether an armed timer fires and short enough not to hide a spin.
    out.push(Cmd::Wait(5_000));
    Ok(out)
}

/// `--pointer KIND,X,Y`, where KIND is `down`, `move`, `up` or `tap`.
fn parse_pointer(v: &str) -> Result<Cmd, String> {
    let mut fields = v.split(',');
    let kind = fields
        .next()
        .ok_or("--pointer needs a kind: down, move, up or tap")?
        .to_ascii_lowercase();
    let need = format!("--pointer {kind} needs ,X,Y");
    let x = parse_coord(fields.next().ok_or(need.as_str())?)?;
    let y = parse_coord(fields.next().ok_or(need.as_str())?)?;
    if fields.next().is_some() {
        return Err(format!("--pointer takes KIND,X,Y: {v}"));
    }
    let down = abi::generated::EVT_POINTER_DOWN;
    Ok(match kind.as_str() {
        "down" => Cmd::Pointer { evt: down, x, y },
        "move" => Cmd::Pointer {
            evt: abi::generated::EVT_POINTER_MOVE,
            x,
            y,
        },
        "up" => Cmd::Pointer {
            evt: abi::generated::EVT_POINTER_UP,
            x,
            y,
        },
        "tap" => Cmd::Tap { x, y },
        other => {
            return Err(format!(
                "unknown --pointer kind {other:?}; use down, move, up or tap"
            ))
        }
    })
}

/// A number, or an SDK name for one. Names are resolved through the generated table so that a
/// command line says what the applet saw rather than what a hex digit happened to be.
fn resolve(v: &str) -> Result<u32, String> {
    if let Some(n) = abi::const_value(v) {
        return Ok(n);
    }
    let n = if let Some(hex) = v.strip_prefix("0x").or(v.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16)
    } else {
        v.parse::<u32>()
    }
    .map_err(|_| format!("neither a number nor a published constant: {v}"))?;
    Ok(n)
}

/// The published name for a number. Header enums share values freely — `AEE_FONT_BOLD` and
/// `AEET_SO_VOICE_IS96` are both `0x8001` — so each call site says which family it is reporting
/// rather than taking whichever entry the table happens to list first.
fn named(value: u32, prefers: &str) -> String {
    abi::CONST_TABLE
        .iter()
        .find(|c| c.value == value && c.name.starts_with(prefers))
        .or_else(|| abi::CONST_TABLE.iter().find(|c| c.value == value))
        .map(|c| c.name.to_string())
        .unwrap_or_else(|| format!("{value:#010x}"))
}

/// `AEEResult` is a small number in 4.x, and the names are in the generated table, so a
/// non-zero return can be read without a lookup.
fn result_name(value: u32) -> String {
    if value == 0 {
        return "AEE_SUCCESS".into();
    }
    abi::CONST_TABLE
        .iter()
        .find(|c| c.value == value && c.name.starts_with("AEE_E"))
        .or_else(|| {
            abi::CONST_TABLE
                .iter()
                .find(|c| c.value == value && c.name.starts_with("AEE_"))
        })
        .map(|c| c.name.to_string())
        .unwrap_or_else(|| format!("AEEResult #{:#x}", value))
}

fn read_word(rt: &Runtime, addr: u32) -> Result<u32, String> {
    rt.cpu.mem.read_u32(addr).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "test/main.rs"]
mod frames_tests;

#[cfg(test)]
#[path = "test/main_coverage.rs"]
mod coverage_tests;
