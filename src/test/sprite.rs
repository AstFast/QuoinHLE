// The sprite engine's own two walks. These are the parts whose every number is published arithmetic
// — the delivered body's layer loop and the tile bounds at — and they need no guest to check.

use super::*;

fn cmd(x: i32, y: i32, size: u8, index: u8, layer: u8, composite: u8) -> Cmd {
    Cmd {
        x,
        y,
        transform: 0,
        matrix: 0,
        index,
        size,
        composite,
        layer,
    }
}

#[test]
fn sprites_are_walked_once_per_layer_and_a_bad_size_only_stains_the_result() {
    // The reference `for` is outside the `while`, so drawing order is by layer and then by
    // list order — a sprite on layer 1 that comes first in the list is still drawn *after* a layer 0
    // one that comes second. A size outside the four sets the result and keeps walking,
    // which is the difference between a refusal and a partly drawn screen with `EBADPARM` on top.
    let bufs = [0x1000, 0x2000, 0x4000, 0x8000];
    let list = [
        cmd(10, 0, 1, 0, 1, 1),
        cmd(0, 0, 0, 0, 0, 0),
        cmd(20, 0, 7, 0, 0, 0),
        cmd(0, 5, 0, 2, 0, 0),
    ];
    let (blits, bad) = plan_sprites(&list, &bufs, 0);
    assert!(bad, "the size outside the four is remembered");
    assert_eq!(
        blits,
        vec![
            Blit {
                x: 0,
                y: 0,
                w: 8,
                h: 8,
                src: 0x1000,
                xs: 0,
                ys: 0,
                transform: 0,
                matrix: 0,
                composite: 0,
            },
            Blit {
                x: 0,
                y: 5,
                w: 8,
                h: 8,
                src: 0x1000,
                xs: 0,
                ys: 16,
                transform: 0,
                matrix: 0,
                composite: 0,
            },
            Blit {
                x: 10,
                y: 0,
                w: 16,
                h: 16,
                src: 0x2000,
                xs: 0,
                ys: 0,
                transform: 0,
                matrix: 0,
                composite: 1,
            },
        ],
        "layer 0 first, and the second row of the buffer is `edge * index`"
    );
}

#[test]
fn bit_15_names_the_caller_s_table_and_without_one_stains_the_answer() {
    // the delivered body: the matrix flag is the one thing the two sprite bodies disagree about
    // besides pixels. With no table the command draws nothing and sets the same sticky `EBADPARM` a bad
    // size sets; with one, the matrix it names is the table plus `unMatrixTransform` entries of eight
    // bytes.
    let matrix = Cmd {
        transform: g::SPRITE_MATRIX_TRANSFORM as u16,
        matrix: 3,
        ..cmd(0, 0, 0, 0, 0, 0)
    };
    let (blits, bad) = plan_sprites(&[matrix], &[0x1000; 4], 0);
    assert!(blits.is_empty(), "nothing is drawn without the table");
    assert!(bad, "and the walk says so");
    let (blits, bad) = plan_sprites(&[matrix], &[0x1000; 4], 0x4000);
    assert!(!bad);
    assert_eq!(blits[0].matrix, 0x4000 + 3 * MATRIX_BYTES);
    assert_eq!(blits[0].transform, g::SPRITE_MATRIX_TRANSFORM as u16);
    // Without the flag the table is not read at all, and the layer test comes first — a hidden
    // command never reaches the matrix branch ( before).
    let (blits, bad) = plan_sprites(&[cmd(0, 0, 0, 0, 0, 0)], &[0x1000; 4], 0x4000);
    assert_eq!(blits[0].matrix, 0);
    assert!(!bad);
    let hidden = Cmd {
        transform: g::SPRITE_MATRIX_TRANSFORM as u16,
        layer: g::SPRITE_LAYER_HIDDEN as u8,
        ..cmd(0, 0, 0, 0, g::SPRITE_LAYER_HIDDEN as u8, 0)
    };
    let (blits, bad) = plan_sprites(&[hidden], &[0x1000; 4], 0);
    assert!(blits.is_empty());
    assert!(
        !bad,
        "a hidden sprite is not an argument the engine complains about"
    );
}

#[test]
fn a_hidden_layer_matches_no_pass() {
    // `unLayer` is 0-3, and the header names 0xFF as "not drawn": the reference pass loop never
    // reaches 0xFF, so the command is walked and dropped.
    let (blits, bad) = plan_sprites(
        &[cmd(0, 0, 0, 0, g::SPRITE_LAYER_HIDDEN as u8, 0)],
        &[0; 4],
        0,
    );
    assert!(blits.is_empty());
    assert!(!bad, "a hidden sprite is a legal thing to be told about");
}

fn map(array: u32, flags: u32, x: i32, y: i32, w: u32, h: u32, size: u8) -> Map {
    Map {
        array,
        flags,
        x,
        y,
        w,
        h,
        size,
    }
}

#[test]
fn a_wrapping_map_starts_one_tile_early_and_indexes_modulo_itself() {
    // 8-pixel tiles, a 4-by-4 map, scrolled 3 pixels right on a 20-by-8 screen: the walk covers the
    // screen from the tile its left edge lands *inside*, so it begins one column early and paints that
    // column at x = -5 (`i * edge + (x & mask)` with `i` starting at -1, and).
    let slots = tile_slots(&map(0x1000, g::MAP_FLAG_WRAP, 3, 0, 2, 2, 0), 20, 8);
    let xs: Vec<i32> = slots.iter().map(|s| s.x).collect();
    assert_eq!(xs, [-5, 3, 11, 19]);
    assert_eq!(
        slots.iter().map(|s| s.at).collect::<Vec<usize>>(),
        [3, 0, 1, 2],
        "the column left of the origin is the map's last one"
    );
    assert!(slots.iter().all(|s| s.y == 0), "y is on a tile edge");
    // The same map scrolled by 11 instead of 3 keeps the *geometry* identical — 11 is 3 plus a whole
    // tile, so it is still one column short of an edge — and moves only the index, because
    // `x >> aTileSizeBits` is subtracted from the column before it is masked.
    let slots = tile_slots(&map(0x1000, g::MAP_FLAG_WRAP, 11, 0, 2, 2, 0), 20, 8);
    assert_eq!(
        slots.iter().map(|s| s.x).collect::<Vec<i32>>(),
        [-5, 3, 11, 19],
        "the pixels do not care how many whole tiles it has gone"
    );
    assert_eq!(
        slots.iter().map(|s| s.at).collect::<Vec<usize>>(),
        [2, 3, 0, 1],
        "the array does"
    );
}

#[test]
fn a_plain_map_is_placed_and_stops_at_its_own_edges() {
    // 16-pixel tiles, a 4-by-2 map on a 50-by-20 screen: four columns fit the width and two rows fit
    // the height, so every element is drawn once, at the map's own order.
    let slots = tile_slots(&map(0x1000, 0, 0, 0, 2, 1, 1), 50, 20);
    assert_eq!(slots.len(), 8);
    assert_eq!(
        slots.iter().map(|s| s.at).collect::<Vec<usize>>(),
        (0..8).collect::<Vec<usize>>()
    );
    assert_eq!(slots[3].x, 48);
    // Nothing outside the map's own rows and columns is drawn, however much screen is left over.
    assert_eq!(
        slots.iter().map(|s| s.x).max().unwrap(),
        48,
        "a fifth column would name an element the map does not have"
    );
    // A map with more columns than the screen has tiles is bounded by the screen: `MIN` at.
    // The bound is on the *columns*, and the loop at is a double one, so the slots it
    // yields are that many columns times the rows the height fits — 256 columns of 16-pixel tiles
    // become 4 on a 50-pixel destination, and 4 columns by the same 2 rows as above is 8 slots.
    let slots = tile_slots(&map(0x1000, 0, 0, 0, 8, 1, 1), 50, 20);
    assert_eq!(
        slots.len(),
        4 * 2,
        "the column bound, times every row it walks"
    );
    assert_eq!(
        slots
            .iter()
            .map(|s| s.x)
            .collect::<std::collections::HashSet<i32>>()
            .len(),
        4,
        "256 columns of tiles, four of them on screen"
    );
    assert_eq!(
        slots.iter().map(|s| s.at).collect::<Vec<usize>>(),
        [0, 1, 2, 3, 256, 257, 258, 259],
        "and the row step is the map's own column count, not the visible one (`:391` multiplies by \
         `(1 << w)`, so a 256-wide map has its second row 256 elements in)"
    );
}

#[test]
fn a_map_pushed_a_whole_map_off_screen_contributes_nothing() {
    // The guard at is `x + (1 << (w + unTileSize + 3)) > 0` and the same for y: a 32-column
    // map of 8-pixel tiles is 256 wide, so 300 pixels left is out and 250 is not.
    assert!(tile_slots(&map(0x1000, 0, -300, 0, 5, 1, 0), 40, 8).is_empty());
    let near = tile_slots(&map(0x1000, 0, -250, 0, 5, 1, 0), 40, 8);
    assert_eq!(
        near.iter().map(|s| s.x).collect::<Vec<i32>>(),
        [-2],
        "back inside, the map's own last column is what reaches the screen"
    );
}

#[test]
fn an_out_of_range_code_answers_no_slots_at_all() {
    // The door refuses these before the walk; the empty answer here is the second line,
    // and it is asserted so the walk can never be reached with an index that would panic.
    assert!(tile_slots(&map(0x1000, 0, 0, 0, 2, 2, 9), 40, 40).is_empty());
    assert!(tile_slots(&map(0x1000, g::MAP_FLAG_WRAP, 0, 0, 99, 2, 0), 40, 40).is_empty());
}
