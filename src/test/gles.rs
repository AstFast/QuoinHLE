use super::*;
use crate::mem::Mem;

fn fixture(w: u32, h: u32) -> (Gles, Egl, u32) {
    let mut egl = Egl::new();
    let id = egl.alloc();
    egl.surfaces
        .insert(id, Surface::new(w, h, SurfaceKind::Pbuffer));
    egl.current = Some(id);
    let mut gl = Gles::new();
    gl.viewport = [0, 0, w as i32, h as i32];
    (gl, egl, id)
}

fn call(gl: &mut Gles, egl: &mut Egl, mem: &mut Mem, name: &str, args: &[u32]) {
    gl.method("IGL", name, args, mem, egl)
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
}

fn bits(v: f32) -> u32 {
    v.to_bits()
}

#[test]
fn a_clear_fills_every_pixel() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(0, 0), [255, 0, 0, 255]);
    assert_eq!(s.pixel(7, 7), [255, 0, 0, 255]);
}

#[test]
fn a_triangle_covers_the_pixels_inside_it() {
    let (mut gl, mut egl, id) = fixture(32, 32);
    let mut mem = Mem::new();
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[0, bits(32.0), 0, bits(32.0), bits(-1.0), bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(0.0, 0.0), (32.0, 0.0), (0.0, 32.0)] {
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    // GL's origin is bottom-left and the drawable keeps that order, so a low x with a low row is
    // inside the lower-left triangle.
    assert_eq!(s.pixel(2, 2), [255, 255, 255, 255]);
    assert_eq!(s.pixel(20, 20), [0, 0, 0, 0]);
}

#[test]
fn depth_test_keeps_the_nearer_triangle() {
    let (mut gl, mut egl, id) = fixture(32, 32);
    let mut mem = Mem::new();
    gl.depth_test = true;
    gl.depth_func = GL_LESS;
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    // Red first at z = 0.5 (nearer under this ortho box), green second at z = 0.
    for (color, z) in [
        ([bits(1.0), 0, 0, bits(1.0)], 0.5),
        ([0, bits(1.0), 0, bits(1.0)], 0.0),
    ] {
        call(&mut gl, &mut egl, &mut mem, "Color4f", &color);
        call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
        for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
            call(
                &mut gl,
                &mut egl,
                &mut mem,
                "Vertex3f",
                &[bits(x), bits(y), bits(z)],
            );
        }
        call(&mut gl, &mut egl, &mut mem, "End", &[]);
    }
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(4, 24), [255, 0, 0, 255]);
}

#[test]
fn a_one_pixel_texture_replaces_the_vertex_colour() {
    let (mut gl, mut egl, id) = fixture(16, 16);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    gl.texture_2d[0] = true;
    gl.tex_env_mode[0] = GL_REPLACE;
    let texel = mem.alloc(4, 4).unwrap();
    mem.write_bytes(texel, &[0, 0, 255, 255]).unwrap();
    let name = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, name]);
    let texture = mem.read_u32(name).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            texel,
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "TexCoord2f",
            &[bits(0.5), bits(0.5)],
        );
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(4, 4), [0, 0, 255, 255]);
}

fn display_fixture() -> (Mem, Display) {
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let mut hle = crate::hle::Hle::new();
    let display =
        Display::new(&mut mem, &mut hle, 16, 16, crate::display::NATIVE_DEPTH).expect("display");
    (mem, display)
}

#[test]
fn qegl_egl10_writes_each_answer_through_its_out_pointer() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let dpy = 0x5000;

    let ret = mem.alloc(4, 4).unwrap();
    egl.method10("GetDisplay", dpy, &[0, ret], &mut mem, &mut display)
        .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), dpy);

    let ret = mem.alloc(4, 4).unwrap();
    egl.method10("Initialize", dpy, &[dpy, 0, 0, ret], &mut mem, &mut display)
        .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), 1);

    let configs = mem.alloc(4, 4).unwrap();
    let count = mem.alloc(4, 4).unwrap();
    let ret = mem.alloc(4, 4).unwrap();
    egl.method10(
        "GetConfigs",
        dpy,
        &[dpy, configs, 1, count, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(count).unwrap(), 1);
    assert_eq!(mem.read_u32(configs).unwrap(), 1);
    assert_eq!(mem.read_u32(ret).unwrap(), 1);
}

#[test]
fn qegl_window_surface_presents_into_the_guest_dib() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let mut gl = Gles::new();
    let dpy = 0x5000;
    let dib = display.device_bitmap();

    let ret = mem.alloc(4, 4).unwrap();
    egl.method10(
        "CreateWindowSurface",
        dpy,
        &[dpy, 1, dib, 0, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    let surface = mem.read_u32(ret).unwrap();
    assert_ne!(surface, 0);

    let ctx_ret = mem.alloc(4, 4).unwrap();
    egl.method10(
        "CreateContext",
        dpy,
        &[dpy, 1, 0, 0, ctx_ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    let ctx = mem.read_u32(ctx_ret).unwrap();
    let ret = mem.alloc(4, 4).unwrap();
    egl.method10(
        "MakeCurrent",
        dpy,
        &[dpy, surface, surface, ctx, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), 1);

    gl.method(
        "IGL",
        "ClearColorx",
        &[65536, 0, 0, 65536],
        &mut mem,
        &mut egl,
    )
    .unwrap();
    gl.method("IGL", "Clear", &[0x4000], &mut mem, &mut egl)
        .unwrap();
    let ret = mem.alloc(4, 4).unwrap();
    egl.method10(
        "SwapBuffers",
        dpy,
        &[dpy, surface, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), 1);

    // Presenting writes the guest's own DIB, which is the device bitmap here.
    let b = display.device;
    assert_eq!(mem.read_u16(b.px).unwrap() & 0xF800, 0xF800);
}

#[test]
fn a_lit_triangle_uses_the_material_diffuse_colour() {
    let (mut gl, mut egl, id) = fixture(32, 32);
    let mut mem = Mem::new();
    gl.lighting = true;
    gl.lights[0].enabled = true;
    // A directional light pointing down +z, so the eye-space light vector is (0, 0, 1) and a
    // vertex whose normal is (0, 0, 1) is lit head-on.
    gl.lights[0].position = [0.0, 0.0, -1.0, 0.0];
    gl.material.diffuse = [1.0, 0.0, 0.0, 1.0];
    gl.material.ambient = [0.0, 0.0, 0.0, 1.0];
    gl.material.emission = [0.0, 0.0, 0.0, 1.0];
    gl.light_model_ambient = [0.0, 0.0, 0.0, 1.0];
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
        call(&mut gl, &mut egl, &mut mem, "Normal3f", &[0, 0, bits(1.0)]);
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(4, 4), [255, 0, 0, 255]);
}

#[test]
fn fog_blends_the_vertex_colour_toward_the_fog_colour() {
    let (mut gl, mut egl, id) = fixture(32, 32);
    let mut mem = Mem::new();
    gl.fog = true;
    gl.fog_mode = GL_LINEAR;
    gl.fog_start = 0.0;
    gl.fog_end = 1.0;
    gl.fog_color = [0.0, 0.0, 1.0, 1.0];
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    // Eye z = -0.5, halfway through the 0..1 fog range and inside the ortho frustum.
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Translatef",
        &[0, 0, bits(-0.5)],
    );
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex3f",
            &[bits(x), bits(y), bits(0.0)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(4, 4), [127, 127, 255, 255]);
}

#[test]
fn a_point_and_a_line_draw_their_pixels() {
    let (mut gl, mut egl, id) = fixture(32, 32);
    let mut mem = Mem::new();
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(0.0),
            bits(32.0),
            bits(0.0),
            bits(32.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "PointSize", &[bits(2.0)]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_POINTS]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Vertex2f",
        &[bits(8.0), bits(8.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_LINES]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Vertex2f",
        &[bits(0.0), bits(0.0)],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Vertex2f",
        &[bits(31.0), bits(31.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(8, 8), [255, 255, 255, 255]);
    assert_eq!(s.pixel(16, 16), [255, 255, 255, 255]);
}

#[test]
fn a_triangle_crossing_the_near_plane_keeps_its_visible_part() {
    let (mut gl, mut egl, id) = fixture(32, 32);
    let mut mem = Mem::new();
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    // z = -2 is behind the near plane (z + w = -1); z = 0 is in front (z + w = 1).
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y, z) in [(0.0, 0.0, 0.0), (1.0, -1.0, -2.0), (1.0, 1.0, -2.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex3f",
            &[bits(x), bits(y), bits(z)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    // The cut keeps the half near x = 0 and drops the part behind the near plane.
    assert_eq!(s.pixel(20, 16), [255, 255, 255, 255]);
}

#[test]
fn copy_tex_image_takes_its_pixels_from_the_drawable() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    gl.texture_2d[0] = true;
    gl.tex_env_mode[0] = GL_REPLACE;
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    let name = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, name]);
    let texture = mem.read_u32(name).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "CopyTexImage2D",
        &[GL_TEXTURE_2D, 0, GL_RGBA, 0, 0, 1, 1, 0],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "TexCoord2f",
            &[bits(0.5), bits(0.5)],
        );
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(2, 2), [255, 0, 0, 255]);
}

#[test]
fn a_second_texture_unit_keeps_its_own_binding() {
    let (mut gl, mut egl, _) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let names = mem.alloc(8, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[2, names]);
    let t0 = mem.read_u32(names).unwrap();
    let t1 = mem.read_u32(names + 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t0],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ActiveTexture",
        &[GL_TEXTURE0 + 1],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t1],
    );
    assert_eq!(gl.active_texture, 1);
    assert_eq!(gl.bound_texture[0], t0);
    assert_eq!(gl.bound_texture[1], t1);
}

#[test]
fn a_colour_mask_keeps_the_masked_channel() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    call(&mut gl, &mut egl, &mut mem, "ColorMask", &[0, 1, 1, 1]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Color4f",
        &[0, bits(1.0), 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    // The red channel was masked, so the cleared red survives under the green triangle.
    assert_eq!(s.pixel(2, 2), [255, 255, 0, 255]);
}

#[test]
fn a_triangle_past_the_viewport_edges_is_clipped_to_the_visible_part() {
    let (mut gl, mut egl, id) = fixture(32, 32);
    let mut mem = Mem::new();
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    // The triangle reaches past left, right and top; the visible part is the triangle between
    // x = -1, x = 1 and y = 1.
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-2.0, 0.0), (2.0, 0.0), (0.0, 2.0)] {
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(16, 20), [255, 255, 255, 255]);
    assert_eq!(s.pixel(0, 0), [0, 0, 0, 0]);
}

#[test]
fn a_compressed_texture_is_decoded_through_the_library() {
    let (mut gl, mut egl, _) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    // One all-zero ETC1 block: 4x4 pixels, 8 bytes.
    let block = [0u8; 8];
    let at = mem.alloc(8, 4).unwrap();
    mem.write_bytes(at, &block).unwrap();
    let name = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, name]);
    let texture = mem.read_u32(name).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "CompressedTexImage2D",
        &[GL_TEXTURE_2D, 0, GL_ETC1_RGB8_OES, 4, 4, 0, 8, at],
    );
    let mut expected = vec![0u32; 16];
    texture2ddecoder::decode_etc1(&block, 4, 4, &mut expected).unwrap();
    let t = gl.textures.get(&texture).unwrap();
    assert_eq!((t.width, t.height), (4, 4));
    for i in 0..16 {
        assert_eq!(t.rgba[i * 4], ((expected[i] >> 16) & 0xff) as u8);
        assert_eq!(t.rgba[i * 4 + 1], ((expected[i] >> 8) & 0xff) as u8);
        assert_eq!(t.rgba[i * 4 + 2], (expected[i] & 0xff) as u8);
    }
}

#[test]
fn a_compressed_sub_image_updates_the_bound_region() {
    let (mut gl, mut egl, _) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let first = [0u8; 8];
    let second = [0xFFu8; 8];
    let a = mem.alloc(8, 4).unwrap();
    let b = mem.alloc(8, 4).unwrap();
    mem.write_bytes(a, &first).unwrap();
    mem.write_bytes(b, &second).unwrap();
    let name = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, name]);
    let texture = mem.read_u32(name).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "CompressedTexImage2D",
        &[GL_TEXTURE_2D, 0, GL_ETC1_RGB8_OES, 4, 4, 0, 8, a],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "CompressedTexSubImage2D",
        &[GL_TEXTURE_2D, 0, 0, 0, 4, 4, GL_ETC1_RGB8_OES, 8, b],
    );
    let mut expected = vec![0u32; 16];
    texture2ddecoder::decode_etc1(&second, 4, 4, &mut expected).unwrap();
    let t = gl.textures.get(&texture).unwrap();
    for i in 0..16 {
        assert_eq!(t.rgba[i * 4], ((expected[i] >> 16) & 0xff) as u8);
        assert_eq!(t.rgba[i * 4 + 1], ((expected[i] >> 8) & 0xff) as u8);
        assert_eq!(t.rgba[i * 4 + 2], (expected[i] & 0xff) as u8);
    }
}

#[test]
fn stencil_test_rejects_pixels_the_reference_does_not_match() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    gl.stencil_test = true;
    gl.stencil_func = GL_EQUAL;
    // The stencil plane starts at zero, so an equality against one rejects every fragment.
    gl.stencil_ref = 1;
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Color4f",
        &[0, bits(1.0), 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(2, 2), [255, 0, 0, 255]);
}

#[test]
fn querying_the_viewport_returns_four_words() {
    let (mut gl, mut egl, _) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let out = mem.alloc(16, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GetIntegerv", &[0x0BA2, out]);
    assert_eq!(mem.read_u32(out).unwrap(), 0);
    assert_eq!(mem.read_u32(out + 4).unwrap(), 0);
    assert_eq!(mem.read_u32(out + 8).unwrap(), 8);
    assert_eq!(mem.read_u32(out + 12).unwrap(), 8);
}

#[test]
fn the_extension_string_names_the_compressed_texture_the_host_decodes() {
    let (mut gl, mut egl, _) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let at = gl
        .method("IGL", "GetString", &[0x1F03], &mut mem, &mut egl)
        .unwrap();
    let bytes = mem.read_bytes(at, 512).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("GL_ATI_texture_compression_atitc"));
    // The same OES extensions the W51K firmware and MP's openglesextension.dll expose.
    assert!(text.contains("GL_OES_vertex_buffer_object"));
    assert!(text.contains("GL_OES_query_matrix"));
    // The framebuffer, blend and point-size work is usable, so a pack is told about it.
    assert!(text.contains("GL_OES_framebuffer_object"));
    assert!(text.contains("GL_OES_blend_func_separate"));
    assert!(text.contains("GL_OES_point_size_array"));
}

#[test]
fn a_polygon_offset_pushes_the_second_triangle_towards_the_eye() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    gl.depth_test = true;
    gl.depth_func = GL_LESS;
    gl.polygon_offset_factor = -1.0;
    gl.polygon_offset_units = -1.0;
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    for (color, offset) in [
        ([bits(1.0), 0, 0, bits(1.0)], false),
        ([0, bits(1.0), 0, bits(1.0)], true),
    ] {
        gl.polygon_offset_fill = offset;
        call(&mut gl, &mut egl, &mut mem, "Color4f", &color);
        call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
        for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
            call(
                &mut gl,
                &mut egl,
                &mut mem,
                "Vertex3f",
                &[bits(x), bits(y), bits(0.0)],
            );
        }
        call(&mut gl, &mut egl, &mut mem, "End", &[]);
    }
    let s = egl.surfaces.get(&id).unwrap();
    // Both triangles sit at the same depth; the negative offset makes green the nearer one.
    assert_eq!(s.pixel(2, 2), [0, 255, 0, 255]);
}

#[test]
fn a_mipmap_min_filter_picks_the_level_the_lod_names() {
    let mut t = Texture::white();
    t.width = 4;
    t.height = 4;
    t.rgba = [255u8, 0, 0, 255].repeat(16);
    t.levels = vec![[0u8, 0, 255, 255].repeat(4)];
    t.min = GL_NEAREST_MIPMAP_NEAREST;
    assert_eq!(t.sample(0.5, 0.5, 0.0), [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(t.sample(0.5, 0.5, 1.0), [0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn tex_sub_image_updates_the_bound_region() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    gl.texture_2d[0] = true;
    gl.tex_env_mode[0] = GL_REPLACE;
    let red = mem.alloc(4, 4).unwrap();
    let blue = mem.alloc(4, 4).unwrap();
    mem.write_bytes(red, &[255, 0, 0, 255]).unwrap();
    mem.write_bytes(blue, &[0, 0, 255, 255]).unwrap();
    let name = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, name]);
    let texture = mem.read_u32(name).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            red,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexSubImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            0,
            0,
            1,
            1,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            blue,
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "TexCoord2f",
            &[bits(0.5), bits(0.5)],
        );
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(2, 2), [0, 0, 255, 255]);
}

#[test]
fn a_second_texture_unit_replaces_the_first() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    gl.texture_2d = [true, true, false, false];
    gl.tex_env_mode = [GL_MODULATE, GL_REPLACE, GL_MODULATE, GL_MODULATE];
    let red = mem.alloc(4, 4).unwrap();
    let green = mem.alloc(4, 4).unwrap();
    mem.write_bytes(red, &[255, 0, 0, 255]).unwrap();
    mem.write_bytes(green, &[0, 255, 0, 255]).unwrap();
    let n0 = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n0]);
    let t0 = mem.read_u32(n0).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t0],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            red,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ActiveTexture",
        &[GL_TEXTURE0 + 1],
    );
    let n1 = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n1]);
    let t1 = mem.read_u32(n1).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t1],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            green,
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "MultiTexCoord4f",
            &[GL_TEXTURE0, bits(0.5), bits(0.5), bits(0.0), bits(1.0)],
        );
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "MultiTexCoord4f",
            &[GL_TEXTURE0 + 1, bits(0.5), bits(0.5), bits(0.0), bits(1.0)],
        );
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    // Unit 0 multiplies the vertex white by red; unit 1 replaces it with green.
    assert_eq!(s.pixel(2, 2), [0, 255, 0, 255]);
}

#[test]
fn a_combine_stage_interpolates_previous_and_texture() {
    let mut gl = Gles::new();
    gl.tex_env_mode[0] = GL_COMBINE;
    gl.combine_rgb[0] = GL_INTERPOLATE;
    gl.combine_alpha[0] = GL_REPLACE;
    gl.src_rgb[0] = [GL_PREVIOUS, GL_TEXTURE, GL_CONSTANT];
    gl.tex_env_color[0] = [0.5, 0.5, 0.5, 1.0];
    let out = gl.combine(0, [1.0, 1.0, 1.0, 1.0], [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(out, [1.0, 0.5, 0.5, 1.0]);
}

#[test]
fn a_dot3_combine_stage_multiplies_the_signed_channels() {
    let mut gl = Gles::new();
    gl.tex_env_mode[0] = GL_COMBINE;
    gl.combine_rgb[0] = GL_DOT3_RGB;
    gl.combine_alpha[0] = GL_REPLACE;
    gl.src_rgb[0] = [GL_PREVIOUS, GL_TEXTURE, GL_CONSTANT];
    gl.src_alpha[0] = [GL_PREVIOUS, GL_TEXTURE, GL_CONSTANT];
    let white = [1.0, 1.0, 1.0, 1.0];
    let black = [0.0, 0.0, 0.0, 0.0];
    // Each channel maps to 2c - 1; white·white = 3, black·white = -3, both clamped.
    assert_eq!(gl.combine(0, white, white), [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(gl.combine(0, black, white), [0.0, 0.0, 0.0, 0.0]);
    // DOT3_RGBA takes alpha from the same dot product, not from COMBINE_ALPHA.
    gl.combine_rgb[0] = GL_DOT3_RGBA;
    assert_eq!(gl.combine(0, white, white), [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn read_pixels_packs_rgb_into_565() {
    let (mut gl, mut egl, id) = fixture(4, 4);
    let _ = id;
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), bits(0.0), bits(0.0), bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    let dst = mem.alloc(16, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ReadPixels",
        &[0, 0, 2, 2, GL_RGB, GL_UNSIGNED_SHORT_5_6_5, dst],
    );
    assert_eq!(mem.read_u16(dst).unwrap(), 0xF800);
}

#[test]
fn a_gles11_buffer_object_keeps_its_size_and_usage() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenBuffers", &[1, n]);
    let buf = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindBuffer",
        &[GL_ARRAY_BUFFER, buf],
    );
    let data = mem.alloc(16, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BufferData",
        &[GL_ARRAY_BUFFER, 16, data, 0x88E4],
    );
    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetBufferParameteriv",
        &[GL_ARRAY_BUFFER, GL_BUFFER_SIZE, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), 16);
    let yes = mem.alloc(4, 4).unwrap();
    mem.write_u32(yes, 0).unwrap();
    assert_eq!(
        gl.method("IGLES11", "IsBuffer", &[buf, yes], &mut mem, &mut egl)
            .unwrap(),
        0,
        "the return is an error code, not the answer"
    );
    assert_eq!(mem.read_u8(yes).unwrap(), 1, "a bound buffer is a buffer");
}

#[test]
fn the_is_queries_answer_through_a_one_byte_pointer_and_return_zero() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);

    // Each flag gets four bytes of its own with the last three marked, because `AEEGLboolean` is
    // `unsigned char`: a word-wide store takes those three with it, and they belong to the caller.
    let flag = |mem: &mut Mem| {
        let at = mem.alloc(4, 4).unwrap();
        mem.write_bytes(at, &[0xef, 0xaa, 0xbb, 0xcc]).unwrap();
        at
    };
    let after = |mem: &Mem, at: u32| mem.read_bytes(at + 1, 3).unwrap();

    // `IsEnabled(IGLES11*, AEEGLenum cap, AEEGLboolean *ret)` — the header.
    let out = flag(&mut mem);
    call(&mut gl, &mut egl, &mut mem, "Enable", &[GL_BLEND]);
    assert_eq!(
        gl.method("IGLES11", "IsEnabled", &[GL_BLEND, out], &mut mem, &mut egl)
            .unwrap(),
        0
    );
    assert_eq!(mem.read_u8(out).unwrap(), 1);
    assert_eq!(after(&mem, out), vec![0xaa, 0xbb, 0xcc]);

    let out = flag(&mut mem);
    call(&mut gl, &mut egl, &mut mem, "Disable", &[GL_BLEND]);
    assert_eq!(
        gl.method("IGLES11", "IsEnabled", &[GL_BLEND, out], &mut mem, &mut egl)
            .unwrap(),
        0
    );
    assert_eq!(mem.read_u8(out).unwrap(), 0);
    assert_eq!(after(&mem, out), vec![0xaa, 0xbb, 0xcc]);

    // `IsTexture(IGLES11*, AEEGLuint texture, AEEGLboolean *ret)` — the header.
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let tex = mem.read_u32(n).unwrap();
    let out = flag(&mut mem);
    assert_eq!(
        gl.method("IGLES11", "IsTexture", &[tex, out], &mut mem, &mut egl)
            .unwrap(),
        0
    );
    assert_eq!(
        mem.read_u8(out).unwrap(),
        1,
        "a generated name is a texture"
    );
    assert_eq!(after(&mem, out), vec![0xaa, 0xbb, 0xcc]);
    let out = flag(&mut mem);
    assert_eq!(
        gl.method(
            "IGLES11",
            "IsTexture",
            &[tex + 100, out],
            &mut mem,
            &mut egl,
        )
        .unwrap(),
        0
    );
    assert_eq!(mem.read_u8(out).unwrap(), 0);
    assert_eq!(after(&mem, out), vec![0xaa, 0xbb, 0xcc]);
}

#[test]
fn the_two_enable_queries_read_one_table() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    // These two caps were answered by two copies of the cap table: one knew GL_LIGHTING, the
    // other GL_TEXTURE_GEN_S. `Enable`/`Disable` sets both state bits, so the pair of queries
    // that ask "is this cap on?" must agree on each cap, in both directions.
    for cap in [GL_LIGHTING, GL_TEXTURE_GEN_S] {
        for on in [true, false] {
            call(
                &mut gl,
                &mut egl,
                &mut mem,
                if on { "Enable" } else { "Disable" },
                &[cap],
            );
            let get = mem.alloc(4, 4).unwrap();
            let is = mem.alloc(4, 4).unwrap();
            mem.write_u32(get, 0).unwrap();
            mem.write_u32(is, 0).unwrap();
            call(&mut gl, &mut egl, &mut mem, "GetBooleanv", &[cap, get]);
            assert_eq!(
                gl.method("IGLES11", "IsEnabled", &[cap, is], &mut mem, &mut egl)
                    .unwrap(),
                0
            );
            assert_eq!(
                mem.read_u8(get).unwrap(),
                mem.read_u8(is).unwrap(),
                "GetBooleanv and IsEnabled disagree about cap {cap:#x} with on={on}"
            );
            assert_eq!(mem.read_u8(is).unwrap(), u8::from(on));
        }
    }

    // Sharing the reader must not widen `glIsEnabled` to every pname `GetBooleanv` can read,
    // nor make an unmodelled cap raise a GL error it did not raise before.
    for cap in [GL_LIGHT_MODEL_TWO_SIDE, 0xdead_beef] {
        let out = mem.alloc(4, 4).unwrap();
        mem.write_u32(out, 0).unwrap();
        assert_eq!(
            gl.method("IGLES11", "IsEnabled", &[cap, out], &mut mem, &mut egl)
                .unwrap(),
            0
        );
        assert_eq!(
            mem.read_u8(out).unwrap(),
            0,
            "cap {cap:#x} is not switchable"
        );
        assert_eq!(
            gl.error, GL_NO_ERROR,
            "IsEnabled stayed silent about {cap:#x}"
        );
    }
}

#[test]
fn the_object_queries_on_the_extension_faces_answer_the_same_way() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let flag = |mem: &mut Mem| {
        let at = mem.alloc(4, 4).unwrap();
        mem.write_bytes(at, &[0xef, 0xaa, 0xbb, 0xcc]).unwrap();
        at
    };

    // `IsFramebufferOES` and `IsRenderbufferOES`.
    for (gen, is, iface) in [
        ("GenFramebuffersOES", "IsFramebufferOES", "IGLES11ExtPak"),
        ("GenRenderbuffersOES", "IsRenderbufferOES", "IGLES11ExtPak"),
    ] {
        let n = mem.alloc(4, 4).unwrap();
        call(&mut gl, &mut egl, &mut mem, gen, &[1, n]);
        let id = mem.read_u32(n).unwrap();
        let out = flag(&mut mem);
        assert_eq!(
            gl.method(iface, is, &[id, out], &mut mem, &mut egl)
                .unwrap(),
            0,
            "{is} returns an error code"
        );
        assert_eq!(mem.read_u8(out).unwrap(), 1, "{gen} made {id} real");
        assert_eq!(mem.read_bytes(out + 1, 3).unwrap(), vec![0xaa, 0xbb, 0xcc]);

        let out = flag(&mut mem);
        assert_eq!(
            gl.method(iface, is, &[id + 100, out], &mut mem, &mut egl)
                .unwrap(),
            0
        );
        assert_eq!(mem.read_u8(out).unwrap(), 0, "{is} of a name nobody made");
    }

    // `IsBufferQUALCOMM` reaches `IsBuffer` through the alias table, same shape.
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenBuffers", &[1, n]);
    let buf = mem.read_u32(n).unwrap();
    // A name `GenBuffers` hands out is not a buffer object until it is bound and given storage
    // (ES 1.1 §2.9), so this answers FALSE before `BufferData` and TRUE after.
    let out = flag(&mut mem);
    assert_eq!(
        gl.method(
            "IGLESImageonExt_V1",
            "IsBufferQUALCOMM",
            &[buf, out],
            &mut mem,
            &mut egl,
        )
        .unwrap(),
        0
    );
    assert_eq!(mem.read_u8(out).unwrap(), 0, "generated, not yet bound");
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindBuffer",
        &[GL_ARRAY_BUFFER, buf],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BufferData",
        &[GL_ARRAY_BUFFER, 4, 0, 0x88E4],
    );
    let out = flag(&mut mem);
    assert_eq!(
        gl.method(
            "IGLESImageonExt_V1",
            "IsBufferQUALCOMM",
            &[buf, out],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        0
    );
    assert_eq!(mem.read_u8(out).unwrap(), 1);
    assert_eq!(mem.read_bytes(out + 1, 3).unwrap(), vec![0xaa, 0xbb, 0xcc]);
}

#[test]
fn a_clip_plane_round_trips_through_fixed() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let eq = mem.alloc(16, 4).unwrap();
    for (i, v) in [1.0f32, 0.0, 0.0, -2.0].iter().enumerate() {
        mem.write_u32(eq + i as u32 * 4, v.to_bits()).unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClipPlanef",
        &[GL_CLIP_PLANE0, eq],
    );
    let out = mem.alloc(16, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetClipPlanex",
        &[GL_CLIP_PLANE0, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), 65536);
    assert_eq!(mem.read_u32(out + 12).unwrap(), (-131072i32) as u32);
}

#[test]
fn a_clip_plane_removes_the_negative_side_of_a_triangle() {
    let (mut gl, mut egl, id) = fixture(16, 16);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    // Keep x >= 0.
    let eq = mem.alloc(16, 4).unwrap();
    for (i, v) in [1.0f32, 0.0, 0.0, 0.0].iter().enumerate() {
        mem.write_u32(eq + i as u32 * 4, v.to_bits()).unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClipPlanef",
        &[GL_CLIP_PLANE0, eq],
    );
    call(&mut gl, &mut egl, &mut mem, "Enable", &[GL_CLIP_PLANE0]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (0.0, 1.0)] {
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    // x = 0.25 stays; x = -0.75 is clipped away.
    assert_eq!(s.pixel(10, 8), [255, 255, 255, 255]);
    assert_eq!(s.pixel(2, 8), [0, 0, 0, 0]);
}

#[test]
fn a_colour_blend_factor_uses_the_source_and_destination_channels() {
    let mut gl = Gles::new();
    gl.blend_src = GL_SRC_COLOR;
    gl.blend_dst = GL_DST_COLOR;
    assert_eq!(gl.blend_factor(0, 0.5, 0.25, 0.5, 1.0), (0.5, 0.25));
    gl.blend_src = GL_SRC_ALPHA_SATURATE;
    gl.blend_dst = GL_ONE_MINUS_SRC_COLOR;
    // min(0.5, 1 - 1.0) = 0.0 and 1 - 0.5 = 0.5.
    assert_eq!(gl.blend_factor(0, 0.5, 0.25, 0.5, 1.0), (0.0, 0.5));
}

#[test]
fn light_and_material_state_can_be_read_back() {
    let mut gl = Gles::new();
    let mut egl = Egl::new();
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let src = mem.alloc(16, 4).unwrap();
    for i in 0..4 {
        mem.write_u32(src + i * 4, bits(0.25)).unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Lightfv",
        &[GL_LIGHT0, GL_AMBIENT, src],
    );
    let out = mem.alloc(16, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetLightfv",
        &[GL_LIGHT0, GL_AMBIENT, out],
    );
    assert_eq!(f32::from_bits(mem.read_u32(out).unwrap()), 0.25);
}

#[test]
fn texture_env_and_parameters_can_be_read_back() {
    let mut gl = Gles::new();
    let mut egl = Egl::new();
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    gl.tex_env_mode[0] = GL_REPLACE;
    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetTexEnviv",
        &[GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), GL_REPLACE);
    let fout = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetTexEnvfv",
        &[GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, fout],
    );
    assert_eq!(
        f32::from_bits(mem.read_u32(fout).unwrap()),
        GL_REPLACE as f32
    );
}

#[test]
fn boolean_and_pointer_queries_read_the_state_back() {
    let mut gl = Gles::new();
    let mut egl = Egl::new();
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    gl.blend = true;
    let out = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GetBooleanv", &[GL_BLEND, out]);
    assert_eq!(mem.read_u8(out).unwrap(), 1);
    gl.client.vertex = Some(Array {
        size: 3,
        ty: GL_FLOAT,
        stride: 0,
        ptr: 0x1234,
    });
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetPointerv",
        &[GL_VERTEX_ARRAY_POINTER, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), 0x1234);
}

#[test]
fn flat_shading_uses_the_last_vertex_colour() {
    let (mut gl, mut egl, id) = fixture(16, 16);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(&mut gl, &mut egl, &mut mem, "ShadeModel", &[GL_FLAT]);
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (r, g, b, x, y) in [
        (1.0, 0.0, 0.0, -1.0, -1.0),
        (0.0, 1.0, 0.0, 1.0, -1.0),
        (0.0, 0.0, 1.0, 0.0, 1.0),
    ] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Color4f",
            &[bits(r), bits(g), bits(b), bits(1.0)],
        );
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    // Flat shading takes the last vertex, which is blue.
    assert_eq!(s.pixel(8, 4), [0, 0, 255, 255]);
}

#[test]
fn point_size_pointer_sets_the_size_per_vertex() {
    let (mut gl, mut egl, id) = fixture(16, 16);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let sizes = mem.alloc(4, 4).unwrap();
    mem.write_u32(sizes, bits(5.0)).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "PointSizePointerOES",
        &[GL_FLOAT, 0, sizes],
    );
    let verts = mem.alloc(8, 4).unwrap();
    mem.write_u32(verts, bits(0.0)).unwrap();
    mem.write_u32(verts + 4, bits(0.0)).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "VertexPointer",
        &[2, GL_FLOAT, 0, verts],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "EnableClientState",
        &[GL_VERTEX_ARRAY],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "DrawArrays",
        &[GL_POINTS, 0, 1],
    );
    let s = egl.surfaces.get(&id).unwrap();
    // The vertex is at the centre with size 5, so it covers x = 10 but not x = 2.
    assert_eq!(s.pixel(10, 8), [255, 255, 255, 255]);
    assert_eq!(s.pixel(2, 8), [0, 0, 0, 0]);
}

#[test]
fn a_luminance_texture_becomes_grey() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let pixel = mem.alloc(4, 4).unwrap();
    mem.write_u8(pixel, 128).unwrap();
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let t = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_LUMINANCE,
            1,
            1,
            0,
            GL_LUMINANCE,
            GL_UNSIGNED_BYTE,
            pixel,
        ],
    );
    let tex = gl.textures.get(&t).unwrap();
    assert_eq!(tex.rgba, vec![128, 128, 128, 255]);
}

#[test]
fn unpack_alignment_sets_the_source_row_pitch() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "PixelStorei",
        &[GL_UNPACK_ALIGNMENT, 1],
    );
    // 3x2 RGB with no padding: row 1 starts at byte 9.
    let px = mem.alloc(32, 4).unwrap();
    let data = [
        255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 255, 255, 255, 255, 0, 0, 255, 0,
    ];
    mem.write_bytes(px, &data).unwrap();
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let t = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGB,
            3,
            2,
            0,
            GL_RGB,
            GL_UNSIGNED_BYTE,
            px,
        ],
    );
    let tex = gl.textures.get(&t).unwrap();
    // Row 1, column 0 is the 10th pixel in RGBA terms.
    assert_eq!(&tex.rgba[12..16], &[0, 255, 255, 255]);
}

#[test]
fn draw_texture_paints_the_bound_texture_on_screen() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let px = mem.alloc(4, 4).unwrap();
    mem.write_bytes(px, &[255, 0, 0, 255]).unwrap();
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let t = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            px,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "DrawTexfOES",
        &[bits(0.0), bits(0.0), bits(0.0), bits(4.0), bits(4.0)],
    );
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(1, 1), [255, 0, 0, 255]);
}

#[test]
fn blend_func_separate_uses_different_alpha_factors() {
    let mut gl = Gles::new();
    gl.blend_src = GL_ONE;
    gl.blend_dst = GL_ZERO;
    gl.blend_src_alpha = GL_ZERO;
    gl.blend_dst_alpha = GL_ONE;
    assert_eq!(gl.blend_factor(0, 0.5, 0.25, 0.5, 1.0), (1.0, 0.0));
    assert_eq!(gl.blend_factor(3, 0.5, 0.25, 0.5, 1.0), (0.0, 1.0));
}

#[test]
fn generate_mipmap_builds_the_halved_levels() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let px = mem.alloc(16, 4).unwrap();
    mem.write_bytes(px, &[255; 16]).unwrap();
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let t = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            2,
            2,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            px,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GenerateMipmapOES",
        &[GL_TEXTURE_2D],
    );
    let tex = gl.textures.get(&t).unwrap();
    // 2x2 -> 1x1 is one generated level.
    assert_eq!(tex.levels.len(), 1);
    assert_eq!(tex.levels[0], vec![255, 255, 255, 255]);
}

#[test]
fn a_paletted_texture_decodes_through_the_palette() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    // PALETTE4_RGB8: u16 mipmap count, 16 palette entries, then 4-bit indices.
    let mut data = vec![1u8, 0u8];
    for _ in 0..16 {
        data.extend([255u8, 0, 0]);
    }
    data.extend([0u8, 0u8]);
    let src = mem.alloc(data.len() as u32, 4).unwrap();
    mem.write_bytes(src, &data).unwrap();
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let t = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "CompressedTexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_PALETTE4_RGB8_OES,
            2,
            2,
            0,
            data.len() as u32,
            src,
        ],
    );
    let tex = gl.textures.get(&t).unwrap();
    assert_eq!(tex.rgba, vec![255, 0, 0, 255].repeat(4));
}

#[test]
fn tex_gen_stores_and_reads_the_object_plane() {
    let mut gl = Gles::new();
    let mut egl = Egl::new();
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let plane = mem.alloc(16, 4).unwrap();
    for (i, v) in [1.0f32, 0.0, 0.0, 0.0].iter().enumerate() {
        mem.write_u32(plane + i as u32 * 4, v.to_bits()).unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexGenfv",
        &[GL_TEXTURE_GEN_S, GL_OBJECT_PLANE, plane],
    );
    let out = mem.alloc(16, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetTexGenfv",
        &[GL_TEXTURE_GEN_S, GL_OBJECT_PLANE, out],
    );
    assert_eq!(f32::from_bits(mem.read_u32(out).unwrap()), 1.0);
}

#[test]
fn a_4444_paletted_texture_expands_the_palette() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    // PALETTE4_RGBA4: u16 mipmap count, 16 two-byte entries, then 4-bit indices.
    let mut data = vec![1u8, 0u8];
    for _ in 0..16 {
        data.extend([0x0Fu8, 0xF0u8]);
    }
    data.extend([0u8, 0u8]);
    let src = mem.alloc(data.len() as u32, 4).unwrap();
    mem.write_bytes(src, &data).unwrap();
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let t = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "CompressedTexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_PALETTE4_RGBA4_OES,
            2,
            2,
            0,
            data.len() as u32,
            src,
        ],
    );
    let tex = gl.textures.get(&t).unwrap();
    assert_eq!(tex.rgba, vec![255, 0, 0, 255].repeat(4));
}

#[test]
fn the_read_format_query_names_rgba_unsigned_byte() {
    let mut gl = Gles::new();
    let mut egl = Egl::new();
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let out = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GetIntegerv", &[0x8B9B, out]);
    assert_eq!(mem.read_u32(out).unwrap(), 6408);
    call(&mut gl, &mut egl, &mut mem, "GetIntegerv", &[0x8B9A, out]);
    assert_eq!(mem.read_u32(out).unwrap(), 3585);
}

#[test]
fn a_framebuffer_renders_into_its_texture() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let t = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            4,
            4,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    let f = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenFramebuffersOES", &[1, f]);
    let fb = mem.read_u32(f).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[0x8D40, fb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferTexture2DOES",
        &[0x8D40, 0x8CE0, GL_TEXTURE_2D, t, 0],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), bits(0.0), bits(0.0), bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[0x8D40, 0],
    );
    let tex = gl.textures.get(&t).unwrap();
    assert_eq!(&tex.rgba[0..4], &[255, 0, 0, 255]);
}

#[test]
fn a_renderbuffer_keeps_its_storage() {
    let mut gl = Gles::new();
    let mut egl = Egl::new();
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenRenderbuffersOES", &[1, n]);
    let rb = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindRenderbufferOES",
        &[GL_RENDERBUFFER_OES, rb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "RenderbufferStorageOES",
        &[GL_RENDERBUFFER_OES, 0x81A5, 4, 4],
    );
    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetRenderbufferParameterivOES",
        &[GL_RENDERBUFFER_OES, GL_RENDERBUFFER_WIDTH_OES, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), 4);
    let yes = mem.alloc(4, 4).unwrap();
    assert_eq!(
        gl.method(
            "IGLES11ExtPak",
            "IsRenderbufferOES",
            &[rb, yes],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        0
    );
    assert_eq!(mem.read_u8(yes).unwrap(), 1);

    // A depth attachment alone is a complete framebuffer: there is no colour attachment,
    // but the specification only calls an empty one missing.
    let f = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenFramebuffersOES", &[1, f]);
    let fb = mem.read_u32(f).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[GL_FRAMEBUFFER_OES, fb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferRenderbufferOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_DEPTH_ATTACHMENT_OES,
            GL_RENDERBUFFER_OES,
            rb,
        ],
    );
    assert_eq!(
        gl.method(
            "IGLES11ExtPak",
            "CheckFramebufferStatusOES",
            &[GL_FRAMEBUFFER_OES],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        GL_FRAMEBUFFER_COMPLETE_OES
    );
}

#[test]
fn read_pixels_writes_a_luminance_byte() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(0.5), bits(0.0), bits(0.0), bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    let dst = mem.alloc(16, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ReadPixels",
        &[0, 0, 2, 2, GL_LUMINANCE, GL_UNSIGNED_BYTE, dst],
    );
    assert_eq!(mem.read_u8(dst).unwrap(), 127);
}

#[test]
fn sphere_map_tex_gen_uses_the_eye_normal() {
    let (mut gl, mut egl, _id) = fixture(16, 16);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let px = mem.alloc(4, 4).unwrap();
    mem.write_bytes(px, &[255, 0, 0, 255]).unwrap();
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let t = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, t],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            px,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexEnvf",
        &[GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, bits(GL_REPLACE as f32)],
    );
    call(&mut gl, &mut egl, &mut mem, "Enable", &[GL_TEXTURE_2D]);
    call(&mut gl, &mut egl, &mut mem, "Enable", &[GL_TEXTURE_GEN_S]);
    // GL_TEXTURE_GEN_T = GL_TEXTURE_GEN_S + 1; the host has no separate constant for it.
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Enable",
        &[GL_TEXTURE_GEN_S + 1],
    );
    for coord in [GL_TEXTURE_GEN_S, GL_TEXTURE_GEN_S + 1] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "TexGeni",
            &[coord, GL_TEXTURE_GEN_MODE, GL_SPHERE_MAP],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Normal3f",
            &[bits(0.0), bits(0.0), bits(1.0)],
        );
        call(&mut gl, &mut egl, &mut mem, "Vertex2f", &[bits(x), bits(y)]);
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&_id).unwrap();
    // A normal of (0, 0, 1) maps to s = t = 0.5, the only texel of the red texture.
    assert_eq!(s.pixel(8, 8), [255, 0, 0, 255]);
}

#[test]
fn a_short_vertex_array_uses_its_type_width_and_default_stride() {
    let (mut gl, mut egl, id) = fixture(32, 32);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    // Two tightly packed (x, y) short vertices: the second begins at byte four, not eight.
    let verts = mem.alloc(8, 4).unwrap();
    for (i, value) in [0i16, 0, 0x4000, 0x4000].iter().enumerate() {
        mem.write_u16(verts + i as u32 * 2, *value as u32).unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "VertexPointer",
        &[2, GL_SHORT, 0, verts],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "EnableClientState",
        &[GL_VERTEX_ARRAY],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[0, bits(32768.0), 0, bits(32768.0), bits(-1.0), bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "PointSize", &[bits(3.0)]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "DrawArrays",
        &[GL_POINTS, 0, 2],
    );
    let s = egl.surfaces.get(&id).unwrap();
    // The first vertex stays at the lower-left corner, the second lands in the centre.
    assert_eq!(s.pixel(0, 0), [255, 255, 255, 255]);
    assert_eq!(s.pixel(16, 16), [255, 255, 255, 255]);
    assert_eq!(s.pixel(8, 8), [0, 0, 0, 0]);
}

#[test]
fn a_short_colour_array_is_normalised_to_the_unit_range() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let verts = mem.alloc(24, 4).unwrap();
    let colours = mem.alloc(24, 4).unwrap();
    for (i, (x, y)) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0)]
        .iter()
        .enumerate()
    {
        mem.write_u32(verts + i as u32 * 8, bits(*x)).unwrap();
        mem.write_u32(verts + i as u32 * 8 + 4, bits(*y)).unwrap();
        for (c, value) in [0x4000u16, 0, 0, 0x7FFF].iter().enumerate() {
            mem.write_u16(colours + i as u32 * 8 + c as u32 * 2, *value as u32)
                .unwrap();
        }
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "VertexPointer",
        &[2, GL_FLOAT, 0, verts],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ColorPointer",
        &[4, GL_SHORT, 0, colours],
    );
    for cap in [GL_VERTEX_ARRAY, GL_COLOR_ARRAY] {
        call(&mut gl, &mut egl, &mut mem, "EnableClientState", &[cap]);
    }
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "DrawArrays",
        &[GL_TRIANGLES, 0, 3],
    );
    let s = egl.surfaces.get(&id).unwrap();
    // 0x4000 / 32767 is almost one half; a raw short would have clamped to full red.
    assert_eq!(s.pixel(2, 2), [127, 0, 0, 255]);
}

#[test]
fn a_four_component_vertex_array_divides_by_its_w() {
    let (mut gl, mut egl, id) = fixture(32, 32);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let verts = mem.alloc(16, 4).unwrap();
    for (i, value) in [0.5f32, 0.0, 0.0, 2.0].iter().enumerate() {
        mem.write_u32(verts + i as u32 * 4, bits(*value)).unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "VertexPointer",
        &[4, GL_FLOAT, 0, verts],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "EnableClientState",
        &[GL_VERTEX_ARRAY],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "PointSize", &[bits(3.0)]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "DrawArrays",
        &[GL_POINTS, 0, 1],
    );
    let s = egl.surfaces.get(&id).unwrap();
    // x = 0.5 divided by w = 2 is 0.25, which is 20 of 32 columns, not 24.
    assert_eq!(s.pixel(20, 16), [255, 255, 255, 255]);
    assert_eq!(s.pixel(24, 16), [0, 0, 0, 0]);
}

#[test]
fn a_depth_renderbuffer_receives_the_depth_plane() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let names = mem.alloc(8, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GenRenderbuffersOES",
        &[1, names],
    );
    let rb = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindRenderbufferOES",
        &[GL_RENDERBUFFER_OES, rb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "RenderbufferStorageOES",
        &[GL_RENDERBUFFER_OES, GL_DEPTH_COMPONENT16_OES, 8, 8],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GenFramebuffersOES",
        &[1, names],
    );
    let fb = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[GL_FRAMEBUFFER_OES, fb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferRenderbufferOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_DEPTH_ATTACHMENT_OES,
            GL_RENDERBUFFER_OES,
            rb,
        ],
    );
    assert_eq!(
        gl.method(
            "IGLES11ExtPak",
            "CheckFramebufferStatusOES",
            &[GL_FRAMEBUFFER_OES],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        GL_FRAMEBUFFER_COMPLETE_OES
    );
    gl.depth_test = true;
    gl.depth_func = GL_LESS;
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex3f",
            &[bits(x), bits(y), bits(0.0)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    // z = 0 is half way through a default depth range, and that is what the renderbuffer holds.
    assert!((gl.renderbuffers.get(&rb).unwrap().depth[8 * 2 + 2] - 0.5).abs() < 1e-6);
    call(&mut gl, &mut egl, &mut mem, "ClearDepthf", &[bits(0.25)]);
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x0100]);
    assert!(gl
        .renderbuffers
        .get(&rb)
        .unwrap()
        .depth
        .iter()
        .all(|d| (*d - 0.25).abs() < 1e-6));
}

#[test]
fn a_framebuffer_reports_missing_and_mismatched_attachments() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let names = mem.alloc(8, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GenFramebuffersOES",
        &[1, names],
    );
    let fb = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[GL_FRAMEBUFFER_OES, fb],
    );
    assert_eq!(
        gl.method(
            "IGLES11ExtPak",
            "CheckFramebufferStatusOES",
            &[GL_FRAMEBUFFER_OES],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT_OES
    );
    // A colour texture and a smaller depth renderbuffer cannot share a framebuffer.
    let tn = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, tn]);
    let texture = mem.read_u32(tn).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            8,
            8,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferTexture2DOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_COLOR_ATTACHMENT0_OES,
            GL_TEXTURE_2D,
            texture,
            0,
        ],
    );
    let rbn = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GenRenderbuffersOES",
        &[1, rbn],
    );
    let rb = mem.read_u32(rbn).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindRenderbufferOES",
        &[GL_RENDERBUFFER_OES, rb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "RenderbufferStorageOES",
        &[GL_RENDERBUFFER_OES, GL_DEPTH_COMPONENT16_OES, 4, 4],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferRenderbufferOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_DEPTH_ATTACHMENT_OES,
            GL_RENDERBUFFER_OES,
            rb,
        ],
    );
    assert_eq!(
        gl.method(
            "IGLES11ExtPak",
            "CheckFramebufferStatusOES",
            &[GL_FRAMEBUFFER_OES],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        GL_FRAMEBUFFER_INCOMPLETE_DIMENSIONS_OES
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "RenderbufferStorageOES",
        &[GL_RENDERBUFFER_OES, GL_DEPTH_COMPONENT16_OES, 8, 8],
    );
    assert_eq!(
        gl.method(
            "IGLES11ExtPak",
            "CheckFramebufferStatusOES",
            &[GL_FRAMEBUFFER_OES],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        GL_FRAMEBUFFER_COMPLETE_OES
    );
}

#[test]
fn an_attachment_query_names_the_texture_and_level() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let names = mem.alloc(8, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, names]);
    let texture = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            8,
            8,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GenFramebuffersOES",
        &[1, names],
    );
    let fb = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[GL_FRAMEBUFFER_OES, fb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferTexture2DOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_COLOR_ATTACHMENT0_OES,
            GL_TEXTURE_2D,
            texture,
            0,
        ],
    );
    let out = mem.alloc(16, 4).unwrap();
    for (pname, want) in [
        (GL_FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE_OES, GL_TEXTURE),
        (GL_FRAMEBUFFER_ATTACHMENT_OBJECT_NAME_OES, texture),
        (GL_FRAMEBUFFER_ATTACHMENT_TEXTURE_LEVEL_OES, 0),
    ] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "GetFramebufferAttachmentParameterivOES",
            &[GL_FRAMEBUFFER_OES, GL_COLOR_ATTACHMENT0_OES, pname, out],
        );
        assert_eq!(mem.read_u32(out).unwrap(), want);
    }
}

#[test]
fn a_colour_only_framebuffer_ignores_the_depth_test() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let names = mem.alloc(8, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, names]);
    let texture = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            8,
            8,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GenFramebuffersOES",
        &[1, names],
    );
    let fb = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[GL_FRAMEBUFFER_OES, fb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferTexture2DOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_COLOR_ATTACHMENT0_OES,
            GL_TEXTURE_2D,
            texture,
            0,
        ],
    );
    gl.depth_test = true;
    gl.depth_func = GL_LESS;
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    // Red is near, green is far. Without a depth attachment the far triangle still paints.
    for (colour, z) in [
        ([bits(1.0), 0, 0, bits(1.0)], -1.0f32),
        ([0, bits(1.0), 0, bits(1.0)], 1.0),
    ] {
        call(&mut gl, &mut egl, &mut mem, "Color4f", &colour);
        call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
        for (x, y) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0)] {
            call(
                &mut gl,
                &mut egl,
                &mut mem,
                "Vertex3f",
                &[bits(x), bits(y), bits(z)],
            );
        }
        call(&mut gl, &mut egl, &mut mem, "End", &[]);
    }
    let t = gl.textures.get(&texture).unwrap();
    assert_eq!(&t.rgba[4 * 4 * 4..4 * 4 * 4 + 4], &[0, 255, 0, 255]);
}

#[test]
fn get_error_reports_the_first_invalid_enum_until_read() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    let error = |gl: &mut Gles, egl: &mut Egl, mem: &mut Mem| {
        gl.method("IGL", "GetError", &[], mem, egl).unwrap()
    };
    call(&mut gl, &mut egl, &mut mem, "Enable", &[0x1234]);
    call(&mut gl, &mut egl, &mut mem, "Disable", &[0x5678]);
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_ENUM);
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_NO_ERROR);

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "PixelStorei",
        &[GL_PACK_ALIGNMENT, 3],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);

    call(&mut gl, &mut egl, &mut mem, "DrawArrays", &[0x9999, 0, 0]);
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_ENUM);
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_NO_ERROR);
}

#[test]
fn texture_calls_reject_unknown_formats_and_enums() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    let error = |gl: &mut Gles, egl: &mut Egl, mem: &mut Mem| {
        gl.method("IGL", "GetError", &[], mem, egl).unwrap()
    };
    call(&mut gl, &mut egl, &mut mem, "BindTexture", &[0x1234, 0]);
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_ENUM);

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            1,
            1,
            0,
            0x1234,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_ENUM);

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            1,
            1,
            1,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexParameteri",
        &[GL_TEXTURE_2D, 0x1234, GL_NEAREST],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_ENUM);

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexParameteri",
        &[GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, 0x1234],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_ENUM);
}

#[test]
fn vector_texture_parameters_reach_the_same_state() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let name = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, name]);
    let texture = mem.read_u32(name).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    let value = mem.alloc(4, 4).unwrap();

    mem.write_u32(value, bits(GL_LINEAR as f32)).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexParameterfv",
        &[GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, value],
    );
    assert_eq!(gl.textures.get(&texture).unwrap().mag, GL_LINEAR);

    mem.write_u32(value, GL_CLAMP_TO_EDGE).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexParameterxv",
        &[GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, value],
    );
    assert_eq!(gl.textures.get(&texture).unwrap().wrap_s, GL_CLAMP_TO_EDGE);

    mem.write_u32(value, GL_NEAREST).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexParameteriv",
        &[GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, value],
    );
    assert_eq!(gl.textures.get(&texture).unwrap().min, GL_NEAREST);
}

#[test]
fn a_spot_light_outside_its_cone_leaves_only_ambient() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    gl.lighting = true;
    gl.lights[0].enabled = true;
    gl.lights[0].position = [0.0, 0.0, 1.0, 1.0];
    gl.lights[0].spot_cutoff = 80.0;
    gl.lights[0].spot_direction = [0.0, 0.0, -1.0];

    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);

    let draw = |gl: &mut Gles, egl: &mut Egl, mem: &mut Mem| {
        call(gl, egl, mem, "Begin", &[GL_TRIANGLES]);
        for (x, y) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0)] {
            call(gl, egl, mem, "Vertex3f", &[bits(x), bits(y), bits(0.0)]);
        }
        call(gl, egl, mem, "End", &[]);
    };
    draw(&mut gl, &mut egl, &mut mem);
    let s = egl.surfaces.get(&id).unwrap();
    // Lighting is per-vertex: each corner sees the light at 0.8 / sqrt(3), so the interpolated
    // centre is 0.8 / sqrt(3) * 255 plus the two ambient terms.
    assert_eq!(s.pixel(2, 2), [127, 127, 127, 255]);

    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    gl.lights[0].spot_direction = [0.0, 0.0, 1.0];
    draw(&mut gl, &mut egl, &mut mem);
    let s = egl.surfaces.get(&id).unwrap();
    // The cone now points away from the vertex, so only the ambient term survives.
    assert_eq!(s.pixel(2, 2), [10, 10, 10, 255]);
}

#[test]
fn matrix_and_binding_queries_answer_the_live_state() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    gl.matrices[0].current = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let out = mem.alloc(64, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetFloatv",
        &[GL_MODELVIEW_MATRIX, out],
    );
    assert_eq!(f32::from_bits(mem.read_u32(out).unwrap()), 1.0);
    assert_eq!(f32::from_bits(mem.read_u32(out + 48).unwrap()), 1.0);
    assert_eq!(f32::from_bits(mem.read_u32(out + 52).unwrap()), 2.0);
    assert_eq!(f32::from_bits(mem.read_u32(out + 56).unwrap()), 3.0);
    assert_eq!(f32::from_bits(mem.read_u32(out + 60).unwrap()), 1.0);

    // The integer entry point does not accept a matrix name.
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetIntegerv",
        &[GL_MODELVIEW_MATRIX, out],
    );
    assert_eq!(
        gl.method("IGL", "GetError", &[], &mut mem, &mut egl)
            .unwrap(),
        GL_INVALID_ENUM
    );

    let names = mem.alloc(8, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, names]);
    let texture = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetIntegerv",
        &[GL_TEXTURE_BINDING_2D, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), texture);

    call(&mut gl, &mut egl, &mut mem, "GenBuffers", &[1, names]);
    let buffer = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindBuffer",
        &[GL_ARRAY_BUFFER, buffer],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetIntegerv",
        &[GL_ARRAY_BUFFER_BINDING, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), buffer);
}

#[test]
fn an_unknown_state_name_raises_invalid_enum() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let out = mem.alloc(16, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GetIntegerv", &[0x1234, out]);
    assert_eq!(
        gl.method("IGL", "GetError", &[], &mut mem, &mut egl)
            .unwrap(),
        GL_INVALID_ENUM
    );
}

fn write_attribs(mem: &mut Mem, at: u32, pairs: &[u32]) {
    for (i, value) in pairs.iter().enumerate() {
        mem.write_u32(at + i as u32 * 4, *value).unwrap();
    }
}

#[test]
fn egl10_choose_config_filters_the_single_config() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let dpy = 0x5000;
    let attribs = mem.alloc(32, 4).unwrap();
    let configs = mem.alloc(4, 4).unwrap();
    let count = mem.alloc(4, 4).unwrap();
    let ret = mem.alloc(4, 4).unwrap();

    // A red channel of eight bits is more than the RGB565 config can promise.
    write_attribs(&mut mem, attribs, &[EGL_RED_SIZE, 8, EGL_NONE]);
    egl.method10(
        "ChooseConfig",
        dpy,
        &[dpy, attribs, configs, 1, count, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(count).unwrap(), 0);
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);

    write_attribs(
        &mut mem,
        attribs,
        &[EGL_RED_SIZE, 4, EGL_DEPTH_SIZE, 16, EGL_NONE],
    );
    egl.method10(
        "ChooseConfig",
        dpy,
        &[dpy, attribs, configs, 1, count, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(count).unwrap(), 1);
    assert_eq!(mem.read_u32(configs).unwrap(), 1);
}

#[test]
fn egl10_get_config_attrib_answers_common_names() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let dpy = 0x5000;
    let value = mem.alloc(4, 4).unwrap();
    let ret = mem.alloc(4, 4).unwrap();
    for (attr, want) in [
        (EGL_CONFIG_ID, 1),
        (EGL_BUFFER_SIZE, 16),
        (EGL_SURFACE_TYPE, 7),
        (EGL_RENDERABLE_TYPE, EGL_OPENGL_ES_BIT),
        (EGL_CONFIG_CAVEAT, EGL_NONE),
        (EGL_MAX_PBUFFER_WIDTH, 2048),
    ] {
        egl.method10(
            "GetConfigAttrib",
            dpy,
            &[dpy, 1, attr, value, ret],
            &mut mem,
            &mut display,
        )
        .unwrap();
        assert_eq!(mem.read_u32(value).unwrap(), want, "attr {attr:#x}");
        assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);
    }
    egl.method10(
        "GetConfigAttrib",
        dpy,
        &[dpy, 1, 0x1234, value, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_FALSE);
}

#[test]
fn egl10_context_queries_follow_make_current() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let dpy = 0x5000;
    let out = mem.alloc(4, 4).unwrap();
    let ret = mem.alloc(4, 4).unwrap();

    egl.method10("GetCurrentContext", dpy, &[out], &mut mem, &mut display)
        .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), EGL_NO_CONTEXT);

    let attribs = mem.alloc(16, 4).unwrap();
    write_attribs(
        &mut mem,
        attribs,
        &[EGL_CONTEXT_CLIENT_VERSION, 1, EGL_NONE],
    );
    egl.method10(
        "CreateContext",
        dpy,
        &[dpy, 1, EGL_NO_CONTEXT, attribs, out, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    let context = mem.read_u32(out).unwrap();
    assert_ne!(context, EGL_NO_CONTEXT);
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);

    egl.method10(
        "MakeCurrent",
        dpy,
        &[dpy, EGL_NO_SURFACE, EGL_NO_SURFACE, context, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);
    egl.method10("GetCurrentContext", dpy, &[out], &mut mem, &mut display)
        .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), context);

    egl.method10(
        "QueryContext",
        dpy,
        &[dpy, context, EGL_CONTEXT_CLIENT_VERSION, out, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), 1);
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);

    egl.method10(
        "DestroyContext",
        dpy,
        &[dpy, context, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);
    egl.method10("GetCurrentContext", dpy, &[out], &mut mem, &mut display)
        .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), EGL_NO_CONTEXT);
}

#[test]
fn egl10_create_context_rejects_an_unknown_attribute() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let dpy = 0x5000;
    let attribs = mem.alloc(16, 4).unwrap();
    write_attribs(&mut mem, attribs, &[0x1234, 1, EGL_NONE]);
    let out = mem.alloc(4, 4).unwrap();
    let ret = mem.alloc(4, 4).unwrap();
    egl.method10(
        "CreateContext",
        dpy,
        &[dpy, 1, EGL_NO_CONTEXT, attribs, out, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), EGL_NO_CONTEXT);
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_FALSE);
}

#[test]
fn scissor_clips_clears_and_draws() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    call(&mut gl, &mut egl, &mut mem, "Scissor", &[2, 2, 4, 4]);
    call(&mut gl, &mut egl, &mut mem, "Enable", &[GL_SCISSOR_TEST]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(1, 1), [0, 0, 0, 0]);
    assert_eq!(s.pixel(3, 3), [255, 0, 0, 255]);

    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex3f",
            &[bits(x), bits(y), bits(0.0)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(1, 1), [0, 0, 0, 0]);
    assert_eq!(s.pixel(3, 3), [255, 255, 255, 255]);
}

#[test]
fn a_texture_object_is_shared_across_units() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let names = mem.alloc(8, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, names]);
    let texture = mem.read_u32(names).unwrap();
    let blue = mem.alloc(4, 4).unwrap();
    mem.write_bytes(blue, &[0, 0, 255, 255]).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            blue,
        ],
    );
    assert_eq!(
        gl.textures.get(&texture).unwrap().rgba,
        vec![0, 0, 255, 255]
    );

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ActiveTexture",
        &[GL_TEXTURE0 + 1],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    // The second unit bound the same object, not a fresh white copy.
    assert_eq!(
        gl.textures.get(&texture).unwrap().rgba,
        vec![0, 0, 255, 255]
    );

    let red = mem.alloc(4, 4).unwrap();
    mem.write_bytes(red, &[255, 0, 0, 255]).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexSubImage2D",
        &[GL_TEXTURE_2D, 0, 0, 0, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, red],
    );
    // An update through unit one is visible in the object both units bind.
    assert_eq!(
        gl.textures.get(&texture).unwrap().rgba,
        vec![255, 0, 0, 255]
    );
    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetIntegerv",
        &[GL_TEXTURE_BINDING_2D, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), texture);
}

#[test]
fn colour_material_replaces_the_diffuse_colour() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    gl.lighting = true;
    gl.lights[0].enabled = true;
    gl.lights[0].position = [0.0, 0.0, -1.0, 0.0];
    gl.material.diffuse = [0.8, 0.8, 0.8, 1.0];
    gl.material.ambient = [0.0, 0.0, 0.0, 1.0];
    gl.light_model_ambient = [0.0, 0.0, 0.0, 1.0];
    gl.color_material = true;
    gl.color_material_face = GL_FRONT_AND_BACK;
    gl.color_material_mode = GL_DIFFUSE;

    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Color4f",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for (x, y) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex3f",
            &[bits(x), bits(y), bits(0.0)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    // The white material was replaced by the red vertex colour, then lit head-on.
    assert_eq!(s.pixel(2, 2), [255, 0, 0, 255]);
}

#[test]
fn colour_material_rejects_an_unknown_mode() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ColorMaterial",
        &[GL_FRONT, 0x1234],
    );
    assert_eq!(
        gl.method("IGL", "GetError", &[], &mut mem, &mut egl)
            .unwrap(),
        GL_INVALID_ENUM
    );
}

#[test]
fn tex_sub_image_updates_the_requested_mip_level() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let names = mem.alloc(8, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, names]);
    let texture = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    let red = mem.alloc(16, 4).unwrap();
    mem.write_bytes(
        red,
        &[
            255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
        ],
    )
    .unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            2,
            2,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            red,
        ],
    );
    let green = mem.alloc(4, 4).unwrap();
    mem.write_bytes(green, &[0, 255, 0, 255]).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            1,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            green,
        ],
    );
    let blue = mem.alloc(4, 4).unwrap();
    mem.write_bytes(blue, &[0, 0, 255, 255]).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexSubImage2D",
        &[
            GL_TEXTURE_2D,
            1,
            0,
            0,
            1,
            1,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            blue,
        ],
    );
    let tex = gl.textures.get(&texture).unwrap();
    assert_eq!(&tex.rgba[0..4], &[255, 0, 0, 255]);
    assert_eq!(&tex.levels[0][0..4], &[0, 0, 255, 255]);
}

#[test]
fn copy_tex_sub_image_uses_the_level_and_offsets_it_was_given() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    let names = mem.alloc(8, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, names]);
    let texture = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    for (level, size) in [(0u32, 2u32), (1, 1)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "TexImage2D",
            &[
                GL_TEXTURE_2D,
                level,
                GL_RGBA,
                size,
                size,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                0,
            ],
        );
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "CopyTexSubImage2D",
        &[GL_TEXTURE_2D, 1, 0, 0, 0, 0, 1, 1],
    );
    let tex = gl.textures.get(&texture).unwrap();
    assert_eq!(&tex.levels[0][0..4], &[255, 0, 0, 255]);
}

#[test]
fn compressed_tex_image_writes_the_requested_mip_level() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let names = mem.alloc(8, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, names]);
    let texture = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            8,
            8,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    let block = mem.alloc(8, 4).unwrap();
    mem.write_bytes(block, &[0u8; 8]).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "CompressedTexImage2D",
        &[GL_TEXTURE_2D, 1, GL_ETC1_RGB8_OES, 4, 4, 0, 8, block],
    );
    let tex = gl.textures.get(&texture).unwrap();
    assert_eq!(tex.levels.len(), 1);
    assert_eq!(tex.levels[0].len(), 4 * 4 * 4);
}

#[test]
fn line_width_draws_a_thicker_line() {
    let (mut gl, mut egl, id) = fixture(16, 16);
    let mut mem = Mem::new();
    call(&mut gl, &mut egl, &mut mem, "LineWidth", &[bits(3.0)]);
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_LINES]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Vertex2f",
        &[bits(-1.0), bits(0.0)],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Vertex2f",
        &[bits(1.0), bits(0.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    // A three-pixel brush covers rows 7..9 around the centre row 8.
    assert_eq!(s.pixel(8, 7), [255, 255, 255, 255]);
    assert_eq!(s.pixel(8, 8), [255, 255, 255, 255]);
    assert_eq!(s.pixel(8, 9), [255, 255, 255, 255]);
    assert_eq!(s.pixel(8, 6), [0, 0, 0, 0]);
}

#[test]
fn two_sided_lighting_flips_a_back_facing_normal() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    gl.lighting = true;
    gl.lights[0].enabled = true;
    gl.lights[0].position = [0.0, 0.0, -1.0, 0.0];
    gl.material.diffuse = [1.0, 1.0, 1.0, 1.0];
    gl.material.ambient = [0.0, 0.0, 0.0, 1.0];
    gl.material.emission = [0.0, 0.0, 0.0, 1.0];
    gl.back_material = gl.material;
    gl.light_model_ambient = [0.0, 0.0, 0.0, 1.0];
    let eye = Vec3::new(0.0, 0.0, -1.0);
    let back = Vec3::new(0.0, 0.0, -1.0);
    assert!(gl.shade(eye, back, [1.0; 4])[0] < 0.01);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "LightModelf",
        &[GL_LIGHT_MODEL_TWO_SIDE, bits(1.0)],
    );
    assert!(gl.shade(eye, back, [1.0; 4])[0] > 0.99);
}

#[test]
fn rescale_normal_is_accepted_as_a_capability() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(&mut gl, &mut egl, &mut mem, "Enable", &[GL_RESCALE_NORMAL]);
    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetBooleanv",
        &[GL_RESCALE_NORMAL, out],
    );
    assert_eq!(mem.read_u8(out).unwrap(), 1);
    assert_eq!(
        gl.method("IGL", "GetError", &[], &mut mem, &mut egl)
            .unwrap(),
        GL_NO_ERROR
    );
}

#[test]
fn the_default_min_filter_is_nearest_mipmap_linear() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let names = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, names]);
    let texture = mem.read_u32(names).unwrap();
    assert_eq!(
        gl.textures.get(&texture).unwrap().min,
        GL_NEAREST_MIPMAP_LINEAR
    );
}

#[test]
fn oversized_counts_and_textures_are_rejected() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let error = |gl: &mut Gles, egl: &mut Egl, mem: &mut Mem| {
        gl.method("IGL", "GetError", &[], mem, egl).unwrap()
    };
    let names = mem.alloc(4, 4).unwrap();

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GenTextures",
        &[MAX_GENERATED_OBJECTS + 1, names],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "DrawArrays",
        &[GL_POINTS, 0, MAX_DRAW_COUNT + 1],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            MAX_TEXTURE_SIZE + 1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);

    call(&mut gl, &mut egl, &mut mem, "GenBuffers", &[1, names]);
    let buffer = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindBuffer",
        &[GL_ARRAY_BUFFER, buffer],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BufferData",
        &[GL_ARRAY_BUFFER, MAX_BUFFER_BYTES + 1, 0, 0x88E4],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);
}

#[test]
fn malformed_calls_report_errors_instead_of_silently_doing_nothing() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let error = |gl: &mut Gles, egl: &mut Egl, mem: &mut Mem| {
        gl.method("IGL", "GetError", &[], mem, egl).unwrap()
    };
    let out = mem.alloc(4, 4).unwrap();

    call(&mut gl, &mut egl, &mut mem, "GetBooleanv", &[0x1234, out]);
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_ENUM);

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ReadPixels",
        &[0, 0, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, 0],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);

    call(&mut gl, &mut egl, &mut mem, "PopMatrix", &[]);
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_STACK_UNDERFLOW);

    let names = mem.alloc(8, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GenFramebuffersOES",
        &[1, names],
    );
    let fb = mem.read_u32(names).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[GL_FRAMEBUFFER_OES, fb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferTexture2DOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_COLOR_ATTACHMENT0_OES,
            GL_TEXTURE_2D,
            0,
            17,
        ],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);
}

#[test]
fn egl_terminate_clears_the_current_context() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let dpy = 0x5000;
    let out = mem.alloc(4, 4).unwrap();
    let ret = mem.alloc(4, 4).unwrap();
    let attribs = mem.alloc(16, 4).unwrap();
    write_attribs(
        &mut mem,
        attribs,
        &[EGL_CONTEXT_CLIENT_VERSION, 1, EGL_NONE],
    );
    egl.method10(
        "CreateContext",
        dpy,
        &[dpy, 1, EGL_NO_CONTEXT, attribs, out, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    let context = mem.read_u32(out).unwrap();
    egl.method10(
        "MakeCurrent",
        dpy,
        &[dpy, EGL_NO_SURFACE, EGL_NO_SURFACE, context, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    egl.method10("Terminate", dpy, &[dpy, ret], &mut mem, &mut display)
        .unwrap();
    egl.method10("GetCurrentContext", dpy, &[out], &mut mem, &mut display)
        .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), EGL_NO_CONTEXT);
}

#[test]
fn egl11_accepts_surface_attributes_and_swap_interval() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let dpy = 0x5000;
    let out = mem.alloc(4, 4).unwrap();
    let ret = mem.alloc(4, 4).unwrap();
    egl.method10(
        "CreatePbufferSurface",
        dpy,
        &[dpy, 1, 0, out],
        &mut mem,
        &mut display,
    )
    .unwrap();
    let surface = mem.read_u32(out).unwrap();
    egl.method11(
        "SurfaceAttrib",
        dpy,
        &[dpy, surface, EGL_SWAP_BEHAVIOR, EGL_BUFFER_DESTROYED, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);
    egl.method11("SwapInterval", dpy, &[dpy, 1, ret], &mut mem, &mut display)
        .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);
    egl.method11(
        "BindTexImage",
        dpy,
        &[dpy, surface, 0, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);
}

#[test]
fn color_buffer_exposes_a_guest_buffer_for_a_pbuffer() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let out = mem.alloc(4, 4).unwrap();
    assert_eq!(
        egl.color_buffer("GetColorBuffer", &[out], &mut mem)
            .unwrap(),
        0
    );
    assert_ne!(mem.read_u32(out).unwrap(), 0);

    let dib = 0x1234_0000;
    egl.surfaces.get_mut(&id).unwrap().target = Some(dib);
    assert_eq!(
        egl.color_buffer("GetColorBuffer", &[out], &mut mem)
            .unwrap(),
        0
    );
    assert_eq!(mem.read_u32(out).unwrap(), dib);
    let _ = &mut gl;
}

#[test]
fn query_matrixx_oes_writes_fixed_matrices_and_a_mask() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    gl.matrices[0].current = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let fixed = mem.alloc(192, 4).unwrap();
    let exponents = mem.alloc(192, 4).unwrap();
    let out = mem.alloc(4, 4).unwrap();
    gl.method(
        "IGLES10Ext",
        "QueryMatrixxOES",
        &[fixed, exponents, out],
        &mut mem,
        &mut egl,
    )
    .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), 0x7);
    assert_eq!(mem.read_u32(fixed + 48).unwrap(), 65536);
    assert_eq!(mem.read_u32(fixed + 52).unwrap(), 131072);
    assert_eq!(mem.read_u32(exponents).unwrap(), 0);
}

#[test]
fn egl_query_surface_answers_the_same_attributes_on_both_faces() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let dpy = 0x5000;
    let out = mem.alloc(4, 4).unwrap();
    let ret = mem.alloc(4, 4).unwrap();
    egl.method10(
        "CreatePbufferSurface",
        dpy,
        &[dpy, 1, 0, out],
        &mut mem,
        &mut display,
    )
    .unwrap();
    let surface = mem.read_u32(out).unwrap();
    for (attr, want) in [
        (EGL_WIDTH, 64),
        (EGL_HEIGHT, 64),
        (EGL_LARGEST_PBUFFER, 64),
        (EGL_SWAP_BEHAVIOR, EGL_BUFFER_DESTROYED),
    ] {
        egl.method(
            "eglQuerySurface",
            dpy,
            &[dpy, surface, attr, out],
            &mut mem,
            &mut display,
        )
        .unwrap();
        assert_eq!(mem.read_u32(out).unwrap(), want, "attr {attr:#x}");
        egl.method10(
            "QuerySurface",
            dpy,
            &[dpy, surface, attr, out, ret],
            &mut mem,
            &mut display,
        )
        .unwrap();
        assert_eq!(mem.read_u32(out).unwrap(), want, "method10 attr {attr:#x}");
        assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);
    }
}

#[test]
fn generate_mipmap_averages_a_2x2_block() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let px = mem.alloc(16, 4).unwrap();
    mem.write_bytes(
        px,
        &[
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
    )
    .unwrap();
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let texture = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            2,
            2,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            px,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GenerateMipmapOES",
        &[GL_TEXTURE_2D],
    );
    let tex = gl.textures.get(&texture).unwrap();
    assert_eq!(tex.levels[0], vec![127, 127, 127, 255]);
}

#[test]
fn read_pixels_packs_rgba_into_4444_and_5551() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), bits(0.5), 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ReadPixels",
        &[0, 0, 1, 1, GL_RGBA, GL_UNSIGNED_SHORT_4_4_4_4, out],
    );
    assert_eq!(mem.read_u16(out).unwrap(), 0xF70F);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ReadPixels",
        &[0, 0, 1, 1, GL_RGBA, GL_UNSIGNED_SHORT_5_5_5_1, out],
    );
    assert_eq!(mem.read_u16(out).unwrap(), 0xFBC1);
}

#[test]
fn tex_image_validates_internal_format_and_level_size() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let error = |gl: &mut Gles, egl: &mut Egl, mem: &mut Mem| {
        gl.method("IGL", "GetError", &[], mem, egl).unwrap()
    };
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            0x1234,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);

    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let texture = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            4,
            4,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            1,
            GL_RGBA,
            4,
            4,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);
}

#[test]
fn clip_plane_indices_and_pointers_are_validated() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let error = |gl: &mut Gles, egl: &mut Egl, mem: &mut Mem| {
        gl.method("IGL", "GetError", &[], mem, egl).unwrap()
    };
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClipPlanef",
        &[GL_CLIP_PLANE0 + 6, 0],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_ENUM);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClipPlanef",
        &[GL_CLIP_PLANE0, 0],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetClipPlanex",
        &[GL_CLIP_PLANE0 + 6, 0],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_ENUM);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetClipPlanex",
        &[GL_CLIP_PLANE0, 0],
    );
    assert_eq!(error(&mut gl, &mut egl, &mut mem), GL_INVALID_VALUE);
}

#[test]
fn point_sprite_replaces_point_colour_with_the_texture() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    gl.texture_2d[0] = true;
    let texel = mem.alloc(4, 4).unwrap();
    mem.write_bytes(texel, &[255, 0, 0, 255]).unwrap();
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let texture = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            texel,
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "PointSize", &[bits(4.0)]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Enable",
        &[GL_POINT_SPRITE_OES],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexEnvi",
        &[GL_POINT_SPRITE_OES, GL_COORD_REPLACE_OES, 1],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_POINTS]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Vertex2f",
        &[bits(0.0), bits(0.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(4, 4), [255, 0, 0, 255]);
    assert_eq!(s.pixel(7, 7), [255, 0, 0, 255]);

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Disable",
        &[GL_POINT_SPRITE_OES],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_POINTS]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Vertex2f",
        &[bits(0.0), bits(0.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(4, 4), [255, 255, 255, 255]);
}

#[test]
fn matrix_palette_transforms_a_skinned_vertex() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let call = |gl: &mut Gles, egl: &mut Egl, mem: &mut Mem, name: &str, args: &[u32]| {
        gl.method("IGL", name, args, mem, egl).unwrap();
    };
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_PROJECTION]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Orthof",
        &[
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
            bits(-1.0),
            bits(1.0),
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Translatef",
        &[bits(1.0), 0, 0],
    );
    call(&mut gl, &mut egl, &mut mem, "CurrentPaletteMatrixOES", &[0]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "LoadPaletteFromModelViewMatrixOES",
        &[],
    );
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Enable",
        &[GL_MATRIX_PALETTE_OES],
    );

    let index = mem.alloc(4, 4).unwrap();
    mem.write_u8(index, 0).unwrap();
    let weight = mem.alloc(4, 4).unwrap();
    mem.write_u32(weight, bits(1.0)).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "MatrixIndexPointerOES",
        &[1, GL_UNSIGNED_BYTE, 0, index],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "WeightPointerOES",
        &[1, GL_FLOAT, 0, weight],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "EnableClientState",
        &[GL_MATRIX_INDEX_ARRAY_OES],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "EnableClientState",
        &[GL_WEIGHT_ARRAY_OES],
    );

    let vertex = mem.alloc(8, 4).unwrap();
    mem.write_u32(vertex, bits(-1.0)).unwrap();
    mem.write_u32(vertex + 4, 0).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "VertexPointer",
        &[2, GL_FLOAT, 0, vertex],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "EnableClientState",
        &[GL_VERTEX_ARRAY],
    );
    call(&mut gl, &mut egl, &mut mem, "PointSize", &[bits(3.0)]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "DrawArrays",
        &[GL_POINTS, 0, 1],
    );

    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(4, 4), [255, 255, 255, 255]);
    assert_eq!(s.pixel(0, 4), [0, 0, 0, 0]);
}

#[test]
fn imageon_aliases_reach_the_core_entries() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    gl.method(
        "IGLESImageonExt",
        "GenBuffersQUALCOMM",
        &[1, n],
        &mut mem,
        &mut egl,
    )
    .unwrap();
    let buffer = mem.read_u32(n).unwrap();
    assert_ne!(buffer, 0);
    gl.method(
        "IGLESImageonExt",
        "BindBufferQUALCOMM",
        &[GL_ARRAY_BUFFER, buffer],
        &mut mem,
        &mut egl,
    )
    .unwrap();
    assert_eq!(gl.bound_buffer_id[0], buffer);
    gl.method(
        "IGLESImageonExt",
        "BufferDataQUALCOMM",
        &[GL_ARRAY_BUFFER, 4, 0, 0x88E4],
        &mut mem,
        &mut egl,
    )
    .unwrap();
    let yes = mem.alloc(4, 4).unwrap();
    assert_eq!(
        gl.method(
            "IGLESImageonExt",
            "IsBufferQUALCOMM",
            &[buffer, yes],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        0
    );
    assert_eq!(mem.read_u8(yes).unwrap(), 1);
    gl.method(
        "IGLESImageonExt",
        "BlendEquationEXT",
        &[GL_FUNC_SUBTRACT],
        &mut mem,
        &mut egl,
    )
    .unwrap();
    assert_eq!(gl.blend_equation, GL_FUNC_SUBTRACT);
}

#[test]
fn egl_get_proc_address_returns_stubs_for_known_egl_names() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let name = mem.alloc(32, 4).unwrap();
    mem.write_bytes(name, b"eglBindTexImage\0").unwrap();
    let address = egl
        .method("eglGetProcAddress", 0, &[name], &mut mem, &mut display)
        .unwrap();
    assert_ne!(address, 0);
    assert_eq!(
        mem.read_u32(address).unwrap() & 0xff00_0000,
        crate::hle::SVC_ARM & 0xff00_0000
    );

    mem.write_bytes(name, b"notAnEglProc\0").unwrap();
    assert_eq!(
        egl.method("eglGetProcAddress", 0, &[name], &mut mem, &mut display)
            .unwrap(),
        0
    );
}

#[test]
fn egl_get_proc_address_returns_stubs_for_gl_extension_names() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let name = mem.alloc(32, 4).unwrap();
    mem.write_bytes(name, b"glPointSizePointerOES\0").unwrap();
    let address = egl
        .method("eglGetProcAddress", 0, &[name], &mut mem, &mut display)
        .unwrap();
    assert_ne!(address, 0);
    assert_eq!(
        mem.read_u32(address).unwrap() & 0xff00_0000,
        crate::hle::SVC_ARM & 0xff00_0000
    );

    mem.write_bytes(name, b"glFramebufferTexture2DOES\0")
        .unwrap();
    let address = egl
        .method("eglGetProcAddress", 0, &[name], &mut mem, &mut display)
        .unwrap();
    assert_ne!(address, 0);

    for extension in [
        "glBindBuffer",
        "glQueryMatrixxOES",
        "glGetRenderbufferParameterivOES",
        "glTexGenivOES",
    ] {
        mem.write_bytes(name, extension.as_bytes()).unwrap();
        mem.write_u8(name + extension.len() as u32, 0).unwrap();
        let address = egl
            .method("eglGetProcAddress", 0, &[name], &mut mem, &mut display)
            .unwrap();
        assert_ne!(address, 0, "{extension}");
    }
}

#[test]
fn point_size_parameters_clamp_the_brush() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "PointParameterf",
        &[GL_POINT_SIZE_MAX, bits(2.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "PointSize", &[bits(4.0)]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_POINTS]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Vertex2f",
        &[bits(0.0), bits(0.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(4, 4), [255, 255, 255, 255]);
    assert_eq!(s.pixel(5, 5), [255, 255, 255, 255]);
    assert_eq!(s.pixel(6, 4), [0, 0, 0, 0]);

    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetFloatv",
        &[GL_POINT_SIZE_MAX, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), bits(2.0));
}

#[test]
fn point_distance_attenuation_shrinks_a_distant_point() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let attenuation = mem.alloc(12, 4).unwrap();
    for (i, value) in [1.0f32, 1.0, 0.0].iter().enumerate() {
        mem.write_u32(attenuation + i as u32 * 4, bits(*value))
            .unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "PointParameterfv",
        &[GL_POINT_DISTANCE_ATTENUATION, attenuation],
    );
    call(&mut gl, &mut egl, &mut mem, "MatrixMode", &[GL_MODELVIEW]);
    call(&mut gl, &mut egl, &mut mem, "LoadIdentity", &[]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Translatef",
        &[0, 0, bits(-1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "PointSize", &[bits(4.0)]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_POINTS]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Vertex2f",
        &[bits(0.0), bits(0.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(6, 4), [255, 255, 255, 255]);
    assert_eq!(s.pixel(7, 4), [0, 0, 0, 0]);
}

#[test]
fn point_fade_threshold_scales_the_alpha() {
    let (mut gl, mut egl, id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "PointParameterf",
        &[GL_POINT_FADE_THRESHOLD_SIZE, bits(2.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "PointSize", &[bits(1.0)]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_POINTS]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Vertex2f",
        &[bits(0.0), bits(0.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(4, 4), [255, 255, 255, 127]);
}

#[test]
fn light_model_local_viewer_can_be_read_back() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "LightModelf",
        &[GL_LIGHT_MODEL_LOCAL_VIEWER, bits(1.0)],
    );
    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetBooleanv",
        &[GL_LIGHT_MODEL_LOCAL_VIEWER, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), 1);
}

#[test]
fn integer_vertex_arrays_are_rejected() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let ptr = mem.alloc(8, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "VertexPointer",
        &[2, 0x1404, 0, ptr],
    );
    assert_eq!(
        gl.method("IGL", "GetError", &[], &mut mem, &mut egl)
            .unwrap(),
        GL_INVALID_ENUM
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ColorPointer",
        &[4, 0x1404, 0, ptr],
    );
    assert_eq!(
        gl.method("IGL", "GetError", &[], &mut mem, &mut egl)
            .unwrap(),
        GL_INVALID_ENUM
    );
}

#[test]
fn egl_surface_attrib_persists_across_both_query_faces() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let dpy = 0x5000;
    let out = mem.alloc(4, 4).unwrap();
    let ret = mem.alloc(4, 4).unwrap();
    egl.method10(
        "CreatePbufferSurface",
        dpy,
        &[dpy, 1, 0, out],
        &mut mem,
        &mut display,
    )
    .unwrap();
    let surface = mem.read_u32(out).unwrap();
    assert_ne!(surface, 0);

    egl.method11(
        "SurfaceAttrib",
        dpy,
        &[dpy, surface, EGL_SWAP_BEHAVIOR, EGL_BUFFER_PRESERVED, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);

    egl.method(
        "eglQuerySurface",
        dpy,
        &[dpy, surface, EGL_SWAP_BEHAVIOR, out],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), EGL_BUFFER_PRESERVED);

    egl.method10(
        "QuerySurface",
        dpy,
        &[dpy, surface, EGL_SWAP_BEHAVIOR, out, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), EGL_BUFFER_PRESERVED);
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);
}

#[test]
fn egl_two_faces_agree_on_config_surface_and_context() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let dpy = 0x5000;
    let out = mem.alloc(4, 4).unwrap();
    let ret = mem.alloc(4, 4).unwrap();

    egl.method(
        "eglGetConfigAttrib",
        dpy,
        &[dpy, 1, EGL_RED_SIZE, out],
        &mut mem,
        &mut display,
    )
    .unwrap();
    let red = mem.read_u32(out).unwrap();
    egl.method10(
        "GetConfigAttrib",
        dpy,
        &[dpy, 1, EGL_RED_SIZE, out, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), red);

    let config = mem.alloc(4, 4).unwrap();
    let count = mem.alloc(4, 4).unwrap();
    egl.method(
        "eglChooseConfig",
        dpy,
        &[dpy, 0, config, 1, count],
        &mut mem,
        &mut display,
    )
    .unwrap();
    let chosen = mem.read_u32(config).unwrap();
    let n = mem.read_u32(count).unwrap();
    egl.method10(
        "ChooseConfig",
        dpy,
        &[dpy, 0, config, 1, count, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(config).unwrap(), chosen);
    assert_eq!(mem.read_u32(count).unwrap(), n);

    egl.method10(
        "CreateContext",
        dpy,
        &[dpy, 1, 0, 0, out, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    let ctx = mem.read_u32(out).unwrap();
    assert_ne!(ctx, 0);

    egl.method(
        "eglQueryContext",
        dpy,
        &[dpy, ctx, EGL_CONTEXT_CLIENT_VERSION, out],
        &mut mem,
        &mut display,
    )
    .unwrap();
    let version = mem.read_u32(out).unwrap();
    egl.method10(
        "QueryContext",
        dpy,
        &[dpy, ctx, EGL_CONTEXT_CLIENT_VERSION, out, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), version);

    egl.method10(
        "MakeCurrent",
        dpy,
        &[dpy, 0, 0, ctx, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(
        egl.method("eglGetCurrentContext", dpy, &[dpy], &mut mem, &mut display)
            .unwrap(),
        ctx
    );
    egl.method10("GetCurrentContext", dpy, &[out], &mut mem, &mut display)
        .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), ctx);
}

#[test]
fn bind_tex_image_updates_the_current_texture() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let surface = egl.alloc();
    let mut data = Surface::new(2, 2, SurfaceKind::Pbuffer);
    data.rgba = vec![
        10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
    ];
    egl.surfaces.insert(surface, data);
    let dpy = 0x5000;
    let ret = mem.alloc(4, 4).unwrap();

    let mut gl = Gles::new();
    gl.texture_2d[0] = true;
    let n = mem.alloc(4, 4).unwrap();
    gl.method("IGL", "GenTextures", &[1, n], &mut mem, &mut egl)
        .unwrap();
    let texture = mem.read_u32(n).unwrap();
    gl.method(
        "IGL",
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
        &mut mem,
        &mut egl,
    )
    .unwrap();
    let pixel = mem.alloc(4, 4).unwrap();
    mem.write_bytes(pixel, &[1, 2, 3, 4]).unwrap();
    gl.method(
        "IGL",
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            pixel,
        ],
        &mut mem,
        &mut egl,
    )
    .unwrap();

    egl.method11(
        "BindTexImage",
        dpy,
        &[dpy, surface, 0, ret],
        &mut mem,
        &mut display,
    )
    .unwrap();
    assert_eq!(mem.read_u32(ret).unwrap(), EGL_TRUE);
    let pending = egl.take_texture_bind().unwrap();
    assert_eq!(pending, surface);
    let (width, height, rgba) = egl.surface_data(surface).unwrap();
    gl.bind_surface_texture(width, height, rgba, EGL_TEXTURE_RGBA);

    let texture = gl.textures.get(&texture).unwrap();
    assert_eq!((texture.width, texture.height), (2, 2));
    assert_eq!(texture.rgba[..4], [10, 20, 30, 255]);
    assert_eq!(texture.levels.len(), 1);
    assert_eq!(texture.levels[0][..4], [55, 65, 75, 255]);
}

#[test]
fn a_depth_texture_attachment_drives_the_depth_test() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);

    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let color = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, color],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            4,
            4,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );

    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let depth = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, depth],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_DEPTH_COMPONENT,
            4,
            4,
            0,
            GL_DEPTH_COMPONENT,
            GL_UNSIGNED_INT,
            0,
        ],
    );

    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenFramebuffersOES", &[1, n]);
    let fb = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[GL_FRAMEBUFFER_OES, fb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferTexture2DOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_COLOR_ATTACHMENT0_OES,
            GL_TEXTURE_2D,
            color,
            0,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferTexture2DOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_DEPTH_ATTACHMENT_OES,
            GL_TEXTURE_2D,
            depth,
            0,
        ],
    );
    assert_eq!(
        gl.method(
            "IGL",
            "CheckFramebufferStatusOES",
            &[GL_FRAMEBUFFER_OES],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        GL_FRAMEBUFFER_COMPLETE_OES
    );

    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetFramebufferAttachmentParameterivOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_DEPTH_ATTACHMENT_OES,
            GL_FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE_OES,
            out,
        ],
    );
    assert_eq!(mem.read_u32(out).unwrap(), GL_TEXTURE);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetFramebufferAttachmentParameterivOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_DEPTH_ATTACHMENT_OES,
            GL_FRAMEBUFFER_ATTACHMENT_OBJECT_NAME_OES,
            out,
        ],
    );
    assert_eq!(mem.read_u32(out).unwrap(), depth);

    call(&mut gl, &mut egl, &mut mem, "Enable", &[GL_DEPTH_TEST]);
    call(&mut gl, &mut egl, &mut mem, "DepthFunc", &[GL_LESS]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[0, 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000 | 0x0100]);

    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Color4f",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    for vertex in [(-1.0, -1.0, -0.5), (1.0, -1.0, -0.5), (0.0, 1.0, -0.5)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex3f",
            &[bits(vertex.0), bits(vertex.1), bits(vertex.2)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);

    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Color4f",
        &[0, bits(1.0), 0, bits(1.0)],
    );
    for vertex in [(1.0, -1.0, 0.5), (-1.0, -1.0, 0.5), (0.0, 1.0, 0.5)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex3f",
            &[bits(vertex.0), bits(vertex.1), bits(vertex.2)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);

    let rgba = &gl.textures.get(&color).unwrap().rgba;
    let i = ((2 * 4 + 2) * 4) as usize;
    assert_eq!(&rgba[i..i + 4], &[255, 0, 0, 255]);
}

#[test]
fn read_pixels_widens_rgba_to_unsigned_short() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), bits(0.5), 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    let out = mem.alloc(8, 2).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ReadPixels",
        &[0, 0, 1, 1, GL_RGBA, GL_UNSIGNED_SHORT, out],
    );
    assert_eq!(mem.read_u16(out).unwrap(), 0xFFFF);
    assert_eq!(mem.read_u16(out + 2).unwrap(), 0x7F7F);
    assert_eq!(mem.read_u16(out + 4).unwrap(), 0);
    assert_eq!(mem.read_u16(out + 6).unwrap(), 0xFFFF);
}

#[test]
fn material_faces_are_separate() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let out = mem.alloc(16, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Materialf",
        &[GL_BACK, GL_SHININESS, bits(16.0)],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetMaterialfv",
        &[GL_BACK, GL_SHININESS, out],
    );
    assert_eq!(f32::from_bits(mem.read_u32(out).unwrap()), 16.0);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetMaterialfv",
        &[GL_FRONT, GL_SHININESS, out],
    );
    assert_eq!(f32::from_bits(mem.read_u32(out).unwrap()), 0.0);

    let src = mem.alloc(16, 4).unwrap();
    for (i, v) in [1.0f32, 0.5, 0.25, 1.0].into_iter().enumerate() {
        mem.write_u32(src + i as u32 * 4, v.to_bits()).unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Materialfv",
        &[GL_BACK, GL_DIFFUSE, src],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetMaterialfv",
        &[GL_BACK, GL_DIFFUSE, out],
    );
    assert_eq!(f32::from_bits(mem.read_u32(out).unwrap()), 1.0);
    assert_eq!(f32::from_bits(mem.read_u32(out + 4).unwrap()), 0.5);
    assert_eq!(f32::from_bits(mem.read_u32(out + 8).unwrap()), 0.25);
    assert_eq!(f32::from_bits(mem.read_u32(out + 12).unwrap()), 1.0);
}

#[test]
fn color_material_face_selects_back_material() {
    let (mut gl, mut egl, _id) = fixture(8, 8);
    let mut mem = Mem::new();
    gl.lighting = true;
    gl.lights[0].enabled = true;
    gl.lights[0].position = [0.0, 0.0, -1.0, 0.0];
    gl.two_sided = true;
    gl.color_material = true;
    gl.color_material_face = GL_BACK;
    gl.color_material_mode = GL_DIFFUSE;
    gl.material.diffuse = [0.1, 0.1, 0.1, 1.0];
    gl.material.ambient = [0.0; 4];
    gl.material.emission = [0.0; 4];
    gl.back_material = gl.material;
    gl.light_model_ambient = [0.0; 4];
    let eye = Vec3::new(0.0, 0.0, -1.0);
    let back = gl.shade(eye, Vec3::new(0.0, 0.0, -1.0), [1.0, 0.0, 0.0, 1.0]);
    let front = gl.shade(eye, Vec3::new(0.0, 0.0, 1.0), [1.0, 0.0, 0.0, 1.0]);
    assert!(back[0] > 0.99);
    assert!(front[0] < 0.2);
    let _ = (&mut mem, &mut egl);
}

#[test]
fn a_cube_face_can_be_a_framebuffer_colour_attachment() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let cube = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_CUBE_MAP, cube],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_CUBE_MAP_POSITIVE_X,
            0,
            GL_RGBA,
            4,
            4,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenFramebuffersOES", &[1, n]);
    let fb = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[GL_FRAMEBUFFER_OES, fb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferTexture2DOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_COLOR_ATTACHMENT0_OES,
            GL_TEXTURE_CUBE_MAP_POSITIVE_X,
            cube,
            0,
        ],
    );
    assert_eq!(
        gl.method(
            "IGL",
            "CheckFramebufferStatusOES",
            &[GL_FRAMEBUFFER_OES],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        GL_FRAMEBUFFER_COMPLETE_OES
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    let face = &gl.textures.get(&cube).unwrap().faces[0];
    assert_eq!(&face[0..4], &[255, 0, 0, 255]);
}

#[test]
fn depth_textures_keep_their_initial_samples() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);

    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let depth = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, depth],
    );
    let src = mem.alloc(8, 2).unwrap();
    for (i, v) in [0u32, 0x8000, 0xFFFF, 0x4000].into_iter().enumerate() {
        mem.write_u16(src + i as u32 * 2, v).unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_DEPTH_COMPONENT,
            2,
            2,
            0,
            GL_DEPTH_COMPONENT,
            GL_UNSIGNED_SHORT,
            src,
        ],
    );
    let samples = &gl.textures.get(&depth).unwrap().depth[0];
    assert!((samples[0] - 0.0).abs() < 1e-4);
    assert!((samples[1] - 0.5).abs() < 1e-3);
    assert!((samples[2] - 1.0).abs() < 1e-4);
    assert!((samples[3] - 0.25).abs() < 1e-3);

    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let ds = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, ds],
    );
    let src = mem.alloc(16, 4).unwrap();
    for (i, v) in [
        0x00FF_FFFF_12u32,
        0x0080_0000_34,
        0x0000_0000_56,
        0x0040_0000_78,
    ]
    .into_iter()
    .enumerate()
    {
        mem.write_u32(src + i as u32 * 4, v).unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_DEPTH_STENCIL_OES,
            2,
            2,
            0,
            GL_DEPTH_STENCIL_OES,
            GL_UNSIGNED_INT_24_8_OES,
            src,
        ],
    );
    let t = gl.textures.get(&ds).unwrap();
    assert!((t.depth[0][0] - 1.0).abs() < 1e-4);
    assert_eq!(t.stencil[0][0], 0x12);
    assert!((t.depth[0][1] - 0.5).abs() < 1e-3);
    assert_eq!(t.stencil[0][1], 0x34);
    assert!((t.depth[0][2] - 0.0).abs() < 1e-4);
    assert_eq!(t.stencil[0][2], 0x56);
    assert!((t.depth[0][3] - 0.25).abs() < 1e-3);
    assert_eq!(t.stencil[0][3], 0x78);
}

#[test]
fn a_cube_map_is_sampled_from_the_major_axis_face() {
    let (mut gl, mut egl, id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let cube = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_CUBE_MAP, cube],
    );
    let pixel = mem.alloc(4, 4).unwrap();
    mem.write_bytes(pixel, &[255, 0, 0, 255]).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_CUBE_MAP_POSITIVE_Z,
            0,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            pixel,
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "Enable", &[GL_TEXTURE_2D]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for vertex in [
        (-1.0, -1.0),
        (1.0, -1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
    ] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "TexCoord3f",
            &[bits(0.0), bits(0.0), bits(1.0)],
        );
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex2f",
            &[bits(vertex.0), bits(vertex.1)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(2, 2), [255, 0, 0, 255]);
}

#[test]
fn a_cube_face_level_can_be_a_framebuffer_attachment() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let cube = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_CUBE_MAP, cube],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_CUBE_MAP_POSITIVE_Z,
            0,
            GL_RGBA,
            2,
            2,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    let pixel = mem.alloc(4, 4).unwrap();
    mem.write_bytes(pixel, &[0, 255, 0, 255]).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_CUBE_MAP_POSITIVE_Z,
            1,
            GL_RGBA,
            1,
            1,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            pixel,
        ],
    );
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenFramebuffersOES", &[1, n]);
    let fb = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[GL_FRAMEBUFFER_OES, fb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferTexture2DOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_COLOR_ATTACHMENT0_OES,
            GL_TEXTURE_CUBE_MAP_POSITIVE_Z,
            cube,
            1,
        ],
    );
    assert_eq!(
        gl.method(
            "IGL",
            "CheckFramebufferStatusOES",
            &[GL_FRAMEBUFFER_OES],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        GL_FRAMEBUFFER_COMPLETE_OES
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[0, 0, bits(1.0), bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    let face = &gl.textures.get(&cube).unwrap().cube_levels[0][4];
    assert_eq!(&face[0..4], &[0, 0, 255, 255]);
}

#[test]
fn cube_generate_mipmap_builds_all_faces() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let cube = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_CUBE_MAP, cube],
    );
    let pixel = mem.alloc(16, 4).unwrap();
    for i in 0..4 {
        mem.write_bytes(pixel + i as u32 * 4, &[255, 0, 0, 255])
            .unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_CUBE_MAP_POSITIVE_Z,
            0,
            GL_RGBA,
            2,
            2,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            pixel,
        ],
    );
    call(&mut gl, &mut egl, &mut mem, "GenerateMipmapOES", &[]);
    let t = gl.textures.get(&cube).unwrap();
    assert_eq!(t.cube_levels.len(), 1);
    assert_eq!(t.cube_levels[0].len(), 6);
    assert_eq!(&t.cube_levels[0][4][..4], &[255, 0, 0, 255]);
    assert_eq!(
        t.sample_cube(0.0, 0.0, 1.0, 1.0, false),
        [1.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn tex_sub_image_updates_the_requested_cube_face() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let cube = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_CUBE_MAP, cube],
    );
    let red = mem.alloc(16, 4).unwrap();
    for i in 0..4 {
        mem.write_bytes(red + i as u32 * 4, &[255, 0, 0, 255])
            .unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_CUBE_MAP_POSITIVE_Z,
            0,
            GL_RGBA,
            2,
            2,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            red,
        ],
    );
    let green = mem.alloc(4, 4).unwrap();
    mem.write_bytes(green, &[0, 255, 0, 255]).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexSubImage2D",
        &[
            GL_TEXTURE_CUBE_MAP_POSITIVE_Z,
            0,
            0,
            0,
            1,
            1,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            green,
        ],
    );
    let face = &gl.textures.get(&cube).unwrap().faces[4];
    assert_eq!(&face[0..4], &[0, 255, 0, 255]);
    assert_eq!(&face[4..8], &[255, 0, 0, 255]);
}

#[test]
fn copy_tex_updates_the_requested_cube_face() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let cube = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_CUBE_MAP, cube],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "CopyTexImage2D",
        &[GL_TEXTURE_CUBE_MAP_POSITIVE_Z, 0, GL_RGBA, 0, 0, 4, 4],
    );
    let face = &gl.textures.get(&cube).unwrap().faces[4];
    assert_eq!(&face[0..4], &[255, 0, 0, 255]);

    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[0, bits(1.0), 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "CopyTexSubImage2D",
        &[GL_TEXTURE_CUBE_MAP_POSITIVE_Z, 0, 0, 0, 0, 0, 1, 1],
    );
    let face = &gl.textures.get(&cube).unwrap().faces[4];
    assert_eq!(&face[0..4], &[0, 255, 0, 255]);
}

#[test]
fn pbuffer_color_buffer_exposes_a_guest_copy() {
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let mut egl = Egl::new();
    let id = egl.alloc();
    let mut surface = Surface::new(2, 2, SurfaceKind::Pbuffer);
    surface.rgba = (0..16).collect();
    egl.surfaces.insert(id, surface);
    egl.current = Some(id);
    let out = mem.alloc(4, 4).unwrap();
    egl.color_buffer("GetColorBuffer", &[out], &mut mem)
        .unwrap();
    let buffer = mem.read_u32(out).unwrap();
    assert_ne!(buffer, 0);
    for i in 0..16 {
        assert_eq!(mem.read_u8(buffer + i).unwrap(), i as u8);
    }

    egl.surfaces.get_mut(&id).unwrap().rgba[0] = 99;
    egl.color_buffer("GetColorBuffer", &[out], &mut mem)
        .unwrap();
    assert_eq!(mem.read_u32(out).unwrap(), buffer);
    assert_eq!(mem.read_u8(buffer).unwrap(), 99);
}

#[test]
fn mirrored_repeat_wrap_is_accepted_and_samples_the_mirrored_texel() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let texture = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexParameteri",
        &[GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_MIRRORED_REPEAT_OES],
    );
    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetTexParameteriv",
        &[GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), GL_MIRRORED_REPEAT_OES);

    let mut t = Texture::white();
    t.width = 2;
    t.height = 1;
    t.rgba = vec![255, 0, 0, 255, 0, 255, 0, 255];
    t.min = GL_NEAREST;
    t.mag = GL_NEAREST;
    t.wrap_s = GL_MIRRORED_REPEAT_OES;
    // 1.25 mirrors to 0.75, which lands on the second texel.
    assert_eq!(t.sample(1.25, 0.5, 0.0), [0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn bind_tex_image_converts_rgb_surface_alpha() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    gl.texture_2d[0] = true;
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let texture = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGB,
            2,
            2,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );
    gl.bind_surface_texture(
        2,
        2,
        vec![1, 2, 3, 0, 4, 5, 6, 0, 7, 8, 9, 0, 10, 11, 12, 0],
        EGL_TEXTURE_RGB,
    );
    let t = gl.textures.get(&texture).unwrap();
    assert_eq!(t.internal, GL_RGB);
    assert_eq!(&t.rgba[..4], &[1, 2, 3, 255]);
    assert_eq!(t.rgba[7], 255);
}

#[test]
fn ati_mesh_list_draws_from_the_uploaded_buffer() {
    let (mut gl, mut egl, id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenBuffers", &[1, n]);
    let vbo = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindBuffer",
        &[GL_ARRAY_BUFFER, vbo],
    );
    let verts = mem.alloc(24, 4).unwrap();
    for (i, v) in [-1.0f32, -1.0, 1.0, -1.0, 0.0, 1.0].into_iter().enumerate() {
        mem.write_u32(verts + i as u32 * 4, v.to_bits()).unwrap();
    }
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BufferDataATI",
        &[GL_ARRAY_BUFFER, 3, 8, 24, verts, 0x88E4],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "VertexPointer",
        &[2, GL_FLOAT, 0, 0],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "EnableClientState",
        &[GL_VERTEX_ARRAY],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Color4f",
        &[bits(1.0), bits(1.0), bits(1.0), bits(1.0)],
    );

    let indices = mem.alloc(6, 2).unwrap();
    for (i, v) in [0u32, 1, 2].into_iter().enumerate() {
        mem.write_u16(indices + i as u32 * 2, v).unwrap();
    }
    let mode = mem.alloc(4, 4).unwrap();
    let count = mem.alloc(4, 4).unwrap();
    let ty = mem.alloc(4, 4).unwrap();
    let index_ptrs = mem.alloc(4, 4).unwrap();
    mem.write_u32(mode, GL_TRIANGLES).unwrap();
    mem.write_u32(count, 3).unwrap();
    mem.write_u32(ty, GL_UNSIGNED_SHORT).unwrap();
    mem.write_u32(index_ptrs, indices).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "MeshListATI",
        &[1, mode, count, ty, index_ptrs],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "DrawVertexBufferObjectATI",
        &[],
    );
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(2, 2), [255, 255, 255, 255]);
}

#[test]
fn sample_coverage_scales_fragment_alpha() {
    let (mut gl, mut egl, id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(&mut gl, &mut egl, &mut mem, "Enable", &[GL_SAMPLE_COVERAGE]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "SampleCoverage",
        &[bits(0.5), 0],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Color4f",
        &[bits(1.0), bits(1.0), bits(1.0), bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    for vertex in [(-1.0, -1.0), (1.0, -1.0), (0.0, 1.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex2f",
            &[bits(vertex.0), bits(vertex.1)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);
    let s = egl.surfaces.get(&id).unwrap();
    assert_eq!(s.pixel(2, 2)[3], 127);

    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetFloatv",
        &[GL_SAMPLE_COVERAGE_VALUE, out],
    );
    assert_eq!(f32::from_bits(mem.read_u32(out).unwrap()), 0.5);
    let flag = mem.alloc(1, 1).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetBooleanv",
        &[GL_SAMPLE_COVERAGE, flag],
    );
    assert_eq!(mem.read_u8(flag).unwrap(), 1);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetIntegerv",
        &[GL_SAMPLES, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), 0);
}

#[test]
fn anisotropy_parameter_and_global_limit_are_exposed() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let texture = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexParameterf",
        &[GL_TEXTURE_2D, GL_TEXTURE_MAX_ANISOTROPY_EXT, bits(8.0)],
    );
    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetTexParameterfv",
        &[GL_TEXTURE_2D, GL_TEXTURE_MAX_ANISOTROPY_EXT, out],
    );
    assert_eq!(f32::from_bits(mem.read_u32(out).unwrap()), 8.0);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetFloatv",
        &[GL_MAX_TEXTURE_MAX_ANISOTROPY_EXT, out],
    );
    assert_eq!(f32::from_bits(mem.read_u32(out).unwrap()), 16.0);
}

#[test]
fn a_cube_depth_texture_attachment_drives_the_depth_test() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);

    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let color = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, color],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            4,
            4,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );

    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let depth = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_CUBE_MAP, depth],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_CUBE_MAP_POSITIVE_Z,
            0,
            GL_DEPTH_COMPONENT,
            4,
            4,
            0,
            GL_DEPTH_COMPONENT,
            GL_UNSIGNED_INT,
            0,
        ],
    );

    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenFramebuffersOES", &[1, n]);
    let fb = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindFramebufferOES",
        &[GL_FRAMEBUFFER_OES, fb],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferTexture2DOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_COLOR_ATTACHMENT0_OES,
            GL_TEXTURE_2D,
            color,
            0,
        ],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "FramebufferTexture2DOES",
        &[
            GL_FRAMEBUFFER_OES,
            GL_DEPTH_ATTACHMENT_OES,
            GL_TEXTURE_CUBE_MAP_POSITIVE_Z,
            depth,
            0,
        ],
    );
    assert_eq!(
        gl.method(
            "IGL",
            "CheckFramebufferStatusOES",
            &[GL_FRAMEBUFFER_OES],
            &mut mem,
            &mut egl
        )
        .unwrap(),
        GL_FRAMEBUFFER_COMPLETE_OES
    );

    call(&mut gl, &mut egl, &mut mem, "Enable", &[GL_DEPTH_TEST]);
    call(&mut gl, &mut egl, &mut mem, "DepthFunc", &[GL_LESS]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[0, 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000 | 0x0100]);

    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Color4f",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    for vertex in [(-1.0, -1.0, -0.5), (1.0, -1.0, -0.5), (0.0, 1.0, -0.5)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex3f",
            &[bits(vertex.0), bits(vertex.1), bits(vertex.2)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);

    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Color4f",
        &[0, bits(1.0), 0, bits(1.0)],
    );
    for vertex in [(1.0, -1.0, 0.5), (-1.0, -1.0, 0.5), (0.0, 1.0, 0.5)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex3f",
            &[bits(vertex.0), bits(vertex.1), bits(vertex.2)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);

    let rgba = &gl.textures.get(&color).unwrap().rgba;
    let i = ((2 * 4 + 2) * 4) as usize;
    assert_eq!(&rgba[i..i + 4], &[255, 0, 0, 255]);
}

#[test]
fn egl_current_is_per_guest_thread() {
    let mut egl = Egl::new();
    egl.contexts.insert(1, 2);
    egl.contexts.insert(2, 2);

    // The primordial thread makes context 1 current.
    assert!(egl.make_current(0, 0, 1));
    assert_eq!(egl.current_context(), Some(1));

    // A worker makes context 2 current, then suspends.
    egl.select_thread(0x2000);
    assert!(egl.make_current(0, 0, 2));
    assert_eq!(egl.current_context(), Some(2));

    // Resuming the primordial thread restores its own current context.
    egl.select_thread(0);
    assert_eq!(egl.current_context(), Some(1));

    // Resuming the worker restores its context, not the primordial one.
    egl.select_thread(0x2000);
    assert_eq!(egl.current_context(), Some(2));

    // A thread that never called make-current has no context of its own.
    egl.select_thread(0x3000);
    assert_eq!(egl.current_context(), None);
}

#[test]
fn atc_rgba_compressed_texture_is_decoded() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let texture = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    let data = mem.alloc(16, 4).unwrap();
    for i in 0..16 {
        mem.write_u8(data + i, 0).unwrap();
    }
    let result = gl
        .method(
            "IGL",
            "CompressedTexImage2D",
            &[
                GL_TEXTURE_2D,
                0,
                GL_ATC_RGBA_EXPLICIT_ALPHA_AMD,
                4,
                4,
                0,
                16,
                data,
            ],
            &mut mem,
            &mut egl,
        )
        .unwrap();
    assert_ne!(result, crate::abi::generated::AEE_EUNSUPPORTED);
    assert_eq!(gl.textures.get(&texture).unwrap().rgba.len(), 64);
}

#[test]
fn logic_op_transforms_fragment_writes() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "ClearColor",
        &[bits(1.0), 0, 0, bits(1.0)],
    );
    call(&mut gl, &mut egl, &mut mem, "Clear", &[0x4000]);
    call(&mut gl, &mut egl, &mut mem, "Enable", &[GL_COLOR_LOGIC_OP]);
    call(&mut gl, &mut egl, &mut mem, "LogicOp", &[GL_COPY_INVERTED]);
    call(&mut gl, &mut egl, &mut mem, "Begin", &[GL_TRIANGLES]);
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "Color4f",
        &[0, bits(1.0), 0, bits(1.0)],
    );
    for vertex in [(-1.0, -1.0, 0.0), (1.0, -1.0, 0.0), (0.0, 1.0, 0.0)] {
        call(
            &mut gl,
            &mut egl,
            &mut mem,
            "Vertex3f",
            &[bits(vertex.0), bits(vertex.1), bits(vertex.2)],
        );
    }
    call(&mut gl, &mut egl, &mut mem, "End", &[]);

    let rgba = &egl.surfaces.values().next().unwrap().rgba;
    let i = ((2 * 4 + 2) * 4) as usize;
    assert_eq!(&rgba[i..i + 4], &[255, 0, 255, 0]);

    let out = mem.alloc(4, 4).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "GetIntegerv",
        &[GL_LOGIC_OP_MODE, out],
    );
    assert_eq!(mem.read_u32(out).unwrap(), GL_COPY_INVERTED);
}

#[test]
fn pbuffer_color_buffer_guest_writes_reach_the_surface() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    let id = egl.alloc();
    egl.surfaces
        .insert(id, Surface::new(4, 4, SurfaceKind::Pbuffer));
    egl.current = Some(id);

    let out = mem.alloc(4, 4).unwrap();
    assert_eq!(
        egl.color_buffer("GetColorBuffer", &[out], &mut mem)
            .unwrap(),
        0
    );
    let buffer = mem.read_u32(out).unwrap();
    for (i, value) in [10u8, 20, 30, 40].iter().enumerate() {
        mem.write_u8(buffer + i as u32, *value).unwrap();
    }

    egl.method("eglSwapBuffers", 0, &[0, id], &mut mem, &mut display)
        .unwrap();
    let rgba = &egl.surfaces.get(&id).unwrap().rgba;
    assert_eq!(&rgba[..4], &[10, 20, 30, 40]);
}

#[test]
fn share_group_snapshot_carries_renderbuffers() {
    let mut source = Gles::new();
    source.renderbuffers.insert(
        0x3000,
        Renderbuffer {
            format: 0x8056,
            width: 4,
            height: 4,
            depth: vec![0.25; 16],
            stencil: vec![7; 16],
        },
    );
    source.next_renderbuffer = 0x3010;

    let shared = source.shareable_snapshot();
    let mut target = Gles::new();
    target.restore_shareable(&shared);

    let rb = target
        .renderbuffers
        .get(&0x3000)
        .expect("shared renderbuffer");
    assert_eq!((rb.width, rb.height), (4, 4));
    assert_eq!(rb.depth[0], 0.25);
    assert_eq!(rb.stencil[0], 7);
    assert_eq!(target.next_renderbuffer, 0x3010);
}

#[test]
fn egl_terminate_clears_every_thread_current() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    egl.contexts.insert(1, 2);
    let surface = egl.alloc();
    egl.surfaces
        .insert(surface, Surface::new(4, 4, SurfaceKind::Pbuffer));

    egl.select_thread(0x1000);
    assert!(egl.make_current(surface, surface, 1));
    egl.select_thread(0x2000);
    assert!(egl.make_current(surface, surface, 1));

    egl.method("eglTerminate", 0, &[0], &mut mem, &mut display)
        .unwrap();

    for thread in [0x1000, 0x2000] {
        egl.select_thread(thread);
        assert_eq!(egl.current_context(), None);
        assert_eq!(egl.current, None);
        assert_eq!(egl.read, None);
    }
}

#[test]
fn egl_destroy_surface_clears_every_thread_surface() {
    let (mut mem, mut display) = display_fixture();
    let mut egl = Egl::new();
    egl.contexts.insert(1, 2);
    let surface = egl.alloc();
    egl.surfaces
        .insert(surface, Surface::new(4, 4, SurfaceKind::Pbuffer));

    egl.select_thread(0x1000);
    assert!(egl.make_current(surface, surface, 1));
    egl.select_thread(0x2000);
    assert!(egl.make_current(surface, surface, 1));

    egl.method(
        "eglDestroySurface",
        0,
        &[0, surface],
        &mut mem,
        &mut display,
    )
    .unwrap();

    for thread in [0x1000, 0x2000] {
        egl.select_thread(thread);
        assert_eq!(egl.current, None);
        assert_eq!(egl.read, None);
        assert_eq!(egl.current_context(), Some(1));
    }
}

#[test]
fn compressed_sub_image_updates_a_paletted_region() {
    let (mut gl, mut egl, _id) = fixture(4, 4);
    let mut mem = Mem::new();
    mem.set_heap(crate::layout::HEAP_BASE, crate::layout::HEAP_LIMIT);
    let n = mem.alloc(4, 4).unwrap();
    call(&mut gl, &mut egl, &mut mem, "GenTextures", &[1, n]);
    let texture = mem.read_u32(n).unwrap();
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "BindTexture",
        &[GL_TEXTURE_2D, texture],
    );
    call(
        &mut gl,
        &mut egl,
        &mut mem,
        "TexImage2D",
        &[
            GL_TEXTURE_2D,
            0,
            GL_RGBA,
            4,
            4,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            0,
        ],
    );

    let mut data = vec![0u8; 58];
    data[2 + 3..2 + 6].copy_from_slice(&[255, 0, 0]);
    data[50..58].fill(0x11);
    let src = mem.alloc(58, 4).unwrap();
    mem.write_bytes(src, &data).unwrap();
    let result = gl
        .method(
            "IGL",
            "CompressedTexSubImage2D",
            &[GL_TEXTURE_2D, 0, 0, 0, 4, 4, GL_PALETTE4_RGB8_OES, 58, src],
            &mut mem,
            &mut egl,
        )
        .unwrap();
    assert_ne!(result, crate::abi::generated::AEE_EUNSUPPORTED);
    assert_eq!(
        &gl.textures.get(&texture).unwrap().rgba[..4],
        &[255, 0, 0, 255]
    );
}
