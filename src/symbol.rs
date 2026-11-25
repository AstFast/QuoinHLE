use crate::abi;
use crate::hle::Hle;
use crate::layout;
use crate::loader::Loaded;

/// The name of whatever lives at `addr`, or the best description the address map can give.
pub fn symbolize(hle: &Hle, module: Option<&Loaded>, addr: u32) -> String {
    // The two addresses worth naming exactly, because they are the two a fault usually carries and
    // neither is inside a region: zero is the null the guest dereferenced, and `HALT` is the
    // sentinel a module returns to.
    if addr == 0 {
        return "NULL".to_string();
    }
    if addr == layout::HALT {
        return "HALT".to_string();
    }
    // The two words the loader writes below the module image: `AEESTDLIB_VER` and the helper table
    // pointer. They are exact addresses rather than a region, and they have to be answered *before*
    // the region scans — the page under a module at [`layout::MODULE_BASE`] falls inside the object
    // region as far as any range check can tell, so a scan would name these two words `<object+…>`.
    if let Some(loaded) = module {
        if loaded.base >= 8 {
            if addr == loaded.base - 8 {
                return "<module+stdver>".to_string();
            }
            if addr == loaded.base - 4 {
                return "<module+helper>".to_string();
            }
        }
    }
    // The stub page: one `SVC` per vtable method, four bytes each, so an address in it is an id.
    // A stub the decoder has no method for is still named as a stub, because "there is a stub here"
    // and "there is a number here" are different findings.
    if (layout::STUB_BASE..layout::VTABLE_BASE).contains(&addr) {
        let offset = addr - layout::STUB_BASE;
        return if offset % 4 == 0 {
            abi::label(offset / 4).to_string()
        } else {
            // Inside a stub rather than at one: `SVC` fetches a word, so nothing should land here.
            format!("stub+{offset:#x} — the middle of a stub, not its address")
        };
    }
    if (layout::VTABLE_BASE..layout::OBJECT_BASE).contains(&addr) {
        if addr == hle.helper_table_addr() {
            return "<AEEHelperFuncs>".to_string();
        }
        return format!("<vtable+{:#x}>", addr - layout::VTABLE_BASE);
    }
    // Objects the host handed out: an address here is one of ours, and the interface is what the
    // caller thinks it is holding. The arena runs up to the guest heap; the module image is a
    // separate, lower region (A carrier loader puts it at 0x8000).
    if (layout::OBJECT_BASE..layout::HEAP_BASE).contains(&addr) {
        if let Some(obj) = hle.object(addr) {
            return format!("<{} object>", obj.iface);
        }
        return format!("<object+{:#x}>", addr - layout::OBJECT_BASE);
    }
    // The module image itself, which is a region like the rest and is checked in the same order.
    if let Some(loaded) = module {
        let (base, len) = (loaded.base, loaded.len);
        if (base..base.wrapping_add(len)).contains(&addr) {
            return format!("module+{:#x}", addr - base);
        }
    }
    if (layout::HEAP_BASE..layout::HEAP_LIMIT).contains(&addr) {
        return format!("heap+{:#x}", addr - layout::HEAP_BASE);
    }
    if (layout::STACK_TOP - layout::STACK_SIZE..layout::STACK_TOP).contains(&addr) {
        return format!("stack-{:#x}", layout::STACK_TOP - addr);
    }
    // Outside every region this host maps. Saying so is the point: an address here is either a
    // guest bug or a region the loader has not been told about, and the two are told apart by
    // whether the number looks like a pointer at all.
    format!("<{addr:#010x} — outside every region this host maps>")
}

/// A `this` pointer's name, which is a shorter question than a general address: it must be an object
/// the host handed out, or it is not a `this` at all.
pub fn this_name(hle: &Hle, addr: u32) -> String {
    match hle.object(addr) {
        Some(obj) => format!("{} object @ {addr:#010x}", obj.iface),
        None => symbolize(hle, None, addr),
    }
}

pub fn parse(module: Option<&Loaded>, text: &str) -> Option<u32> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("NULL") {
        return Some(0);
    }
    if text.eq_ignore_ascii_case("HALT") {
        return Some(layout::HALT);
    }
    if let Some(rest) = text.strip_prefix("module+") {
        return Some(module?.base.wrapping_add(number(rest)?));
    }
    if let Some(rest) = text.strip_prefix("heap+") {
        return Some(layout::HEAP_BASE.wrapping_add(number(rest)?));
    }
    if let Some(rest) = text.strip_prefix("stack-") {
        return Some(layout::STACK_TOP.wrapping_sub(number(rest)?));
    }
    // `Iface::Method`, which is what a call log and a listing both print. What the name denotes is
    // where the dispatch is entered, so that is the address it comes back as.
    if let Some((iface, method)) = text.split_once("::") {
        return abi::method_of(iface, method).map(|id| layout::STUB_BASE + id * 4);
    }
    number(text)
}

/// A literal: `0x…` for hex, decimal otherwise. Both are accepted everywhere the command line takes
/// a number, and this is the one place that spells the rule out.
pub fn number(text: &str) -> Option<u32> {
    let text = text.trim();
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

#[cfg(test)]
#[path = "test/symbol.rs"]
mod tests;
