// Spike S6: the compositor's pipelines (graphics design, GR3), compiled from
// source at run time, so no Xcode toolchain is needed (S2).
//
// Every draw is instanced: four vertices per quad, generated from vertex_id
// (a triangle strip), and one instance per display item. The vertex shader
// reads the item through an index buffer that lists, for the tile being
// drawn, the items that touch it. Positions are in layer pixels, y down.
// Colours are premultiplied, for One / OneMinusSourceAlpha blending.
//
// The structs must match `platform::gpu` byte for byte: float4 fields are
// 16-byte aligned, float2 8-byte.

#include <metal_stdlib>
using namespace metal;

struct Frame {
    float2 origin; // the tile's top-left corner, in layer pixels
    float2 size;   // the tile's size, in pixels
};

struct Rect {
    float4 rect;       // x, y, width, height
    float4 color;      // premultiplied RGBA
    float4 radii;      // corner radii: top left, top right, bottom right, bottom left
    float4 clip;       // a rounded clip: x, y, width, height; width <= 0 for none
    float4 clip_radii;
    float4 extra;      // x: border width (0 fills); y: rotation about the centre, radians
};

struct Shadow {
    float4 rect;   // the shadow's box, offset already applied
    float4 color;  // premultiplied
    float4 params; // x: corner radius; y: blur sigma
};

struct Gradient {
    float4 rect;
    float4 color0; // premultiplied, at the line's start
    float4 color1; // premultiplied, at its end
    float4 line;   // start point (xy) and end point (zw), in layer pixels
    float4 radii;
};

struct Image {
    float4 rect;
    float4 uv;    // u0, v0, u1, v1, normalized
    float4 radii;
    float4 extra; // x: opacity
};

struct Glyph {
    float4 rect;  // whole pixels
    float4 uv;    // u0, v0, u1, v1, normalized
    float4 color; // premultiplied
};

struct Cover {
    float4 rect;  // a path's bounding box
    float4 color; // premultiplied
};

struct Varyings {
    float4 position [[position]];
    float2 pixel;              // layer pixels
    uint item [[flat]];
};

static float2 corner_of(uint vid) {
    return float2(float(vid & 1u), float(vid >> 1u)); // (0,0) (1,0) (0,1) (1,1)
}

static float4 to_clip(float2 pixel, constant Frame& frame) {
    float2 local = (pixel - frame.origin) / frame.size;
    return float4(local.x * 2.0 - 1.0, 1.0 - local.y * 2.0, 0.0, 1.0);
}

// Signed distance from p to a box centred on the origin with half-size
// `half_size` and corner radii r (top left, top right, bottom right, bottom
// left; y down).
static float rounded_box(float2 p, float2 half_size, float4 r) {
    float radius = p.x > 0.0 ? (p.y > 0.0 ? r.z : r.y) : (p.y > 0.0 ? r.w : r.x);
    radius = min(radius, min(half_size.x, half_size.y));
    float2 q = abs(p) - half_size + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - radius;
}

// Coverage of the pixel centred at p by the rounded rectangle rect: analytic
// anti-aliasing over one pixel.
static float coverage(float2 p, float4 rect, float4 radii) {
    float2 half_size = rect.zw * 0.5;
    float d = rounded_box(p - (rect.xy + half_size), half_size, radii);
    return clamp(0.5 - d, 0.0, 1.0);
}

// Winitzki's approximation of the error function (a = 0.147), within about
// 1e-3 of it: enough for shadows.
static float erf_approx(float x) {
    float a = 0.147;
    float x2 = x * x;
    float t = 1.0 - exp(-x2 * (1.2732395 + a * x2) / (1.0 + a * x2));
    return sign(x) * sqrt(max(t, 0.0));
}

// --- Rectangles, rounded rectangles, borders, rounded clips ---------------

vertex Varyings vs_rect(uint vid [[vertex_id]], uint iid [[instance_id]],
                        constant Frame& frame [[buffer(0)]],
                        const device Rect* items [[buffer(1)]],
                        const device uint* ids [[buffer(2)]]) {
    uint item = ids[iid];
    Rect r = items[item];
    float2 corner = corner_of(vid);
    float2 centre = r.rect.xy + r.rect.zw * 0.5;
    // A rotated item covers its circumscribed square; one pixel of margin for
    // anti-aliasing.
    float2 half_extent = r.extra.y != 0.0 ? float2(length(r.rect.zw) * 0.5) : r.rect.zw * 0.5;
    float2 pixel = centre + (corner * 2.0 - 1.0) * (half_extent + 1.0);
    Varyings out;
    out.position = to_clip(pixel, frame);
    out.pixel = pixel;
    out.item = item;
    return out;
}

fragment float4 fs_rect(Varyings in [[stage_in]], const device Rect* items [[buffer(1)]]) {
    Rect r = items[in.item];
    float2 p = in.pixel;
    if (r.extra.y != 0.0) {
        float2 centre = r.rect.xy + r.rect.zw * 0.5;
        float s = sin(-r.extra.y), c = cos(-r.extra.y);
        float2 d = p - centre;
        p = centre + float2(c * d.x - s * d.y, s * d.x + c * d.y);
    }
    float a = coverage(p, r.rect, r.radii);
    float border = r.extra.x;
    if (border > 0.0) {
        float4 inner = float4(r.rect.xy + border, r.rect.zw - 2.0 * border);
        float inside = inner.z > 0.0 && inner.w > 0.0
            ? coverage(p, inner, max(r.radii - border, 0.0)) : 0.0;
        a = max(a - inside, 0.0);
    }
    if (r.clip.z > 0.0) {
        a *= coverage(in.pixel, r.clip, r.clip_radii);
    }
    return r.color * a;
}

// The same rectangles, opaque and unrounded: drawn without blending, so the
// GPU's hidden surface removal can skip what they cover (GR4).
fragment float4 fs_rect_opaque(Varyings in [[stage_in]], const device Rect* items [[buffer(1)]]) {
    return items[in.item].color;
}

// --- Box shadows -------------------------------------------------------------

vertex Varyings vs_shadow(uint vid [[vertex_id]], uint iid [[instance_id]],
                          constant Frame& frame [[buffer(0)]],
                          const device Shadow* items [[buffer(1)]],
                          const device uint* ids [[buffer(2)]]) {
    uint item = ids[iid];
    Shadow s = items[item];
    float margin = 3.0 * s.params.y + 1.0;
    float2 pixel = s.rect.xy - margin + corner_of(vid) * (s.rect.zw + 2.0 * margin);
    Varyings out;
    out.position = to_clip(pixel, frame);
    out.pixel = pixel;
    out.item = item;
    return out;
}

fragment float4 fs_shadow(Varyings in [[stage_in]], const device Shadow* items [[buffer(1)]]) {
    Shadow s = items[in.item];
    float2 half_size = s.rect.zw * 0.5;
    float d = rounded_box(in.pixel - (s.rect.xy + half_size), half_size, float4(s.params.x));
    float sigma = max(s.params.y, 0.25);
    // A Gaussian-blurred edge at signed distance d: exact for straight edges.
    float a = 0.5 - 0.5 * erf_approx(d / (1.41421356 * sigma));
    return s.color * a;
}

// --- Linear gradients ----------------------------------------------------------

vertex Varyings vs_gradient(uint vid [[vertex_id]], uint iid [[instance_id]],
                            constant Frame& frame [[buffer(0)]],
                            const device Gradient* items [[buffer(1)]],
                            const device uint* ids [[buffer(2)]]) {
    uint item = ids[iid];
    Gradient g = items[item];
    float2 pixel = g.rect.xy - 1.0 + corner_of(vid) * (g.rect.zw + 2.0);
    Varyings out;
    out.position = to_clip(pixel, frame);
    out.pixel = pixel;
    out.item = item;
    return out;
}

fragment float4 fs_gradient(Varyings in [[stage_in]], const device Gradient* items [[buffer(1)]]) {
    Gradient g = items[in.item];
    float2 axis = g.line.zw - g.line.xy;
    float t = clamp(dot(in.pixel - g.line.xy, axis) / max(dot(axis, axis), 1e-6), 0.0, 1.0);
    return mix(g.color0, g.color1, t) * coverage(in.pixel, g.rect, g.radii);
}

// --- Images ----------------------------------------------------------------------

vertex Varyings vs_image(uint vid [[vertex_id]], uint iid [[instance_id]],
                         constant Frame& frame [[buffer(0)]],
                         const device Image* items [[buffer(1)]],
                         const device uint* ids [[buffer(2)]]) {
    uint item = ids[iid];
    Image m = items[item];
    float2 pixel = m.rect.xy + corner_of(vid) * m.rect.zw;
    Varyings out;
    out.position = to_clip(pixel, frame);
    out.pixel = pixel;
    out.item = item;
    return out;
}

fragment float4 fs_image(Varyings in [[stage_in]], const device Image* items [[buffer(1)]],
                         texture2d<float> atlas [[texture(0)]]) {
    constexpr sampler linear(coord::normalized, filter::linear, address::clamp_to_edge);
    Image m = items[in.item];
    float2 t = (in.pixel - m.rect.xy) / m.rect.zw;
    float2 uv = mix(m.uv.xy, m.uv.zw, t);
    return atlas.sample(linear, uv) * (m.extra.x * coverage(in.pixel, m.rect, m.radii));
}

// --- Glyphs ------------------------------------------------------------------------

vertex Varyings vs_glyph(uint vid [[vertex_id]], uint iid [[instance_id]],
                         constant Frame& frame [[buffer(0)]],
                         const device Glyph* items [[buffer(1)]],
                         const device uint* ids [[buffer(2)]]) {
    uint item = ids[iid];
    Glyph g = items[item];
    float2 pixel = g.rect.xy + corner_of(vid) * g.rect.zw;
    Varyings out;
    out.position = to_clip(pixel, frame);
    out.pixel = pixel;
    out.item = item;
    return out;
}

fragment float4 fs_glyph(Varyings in [[stage_in]], const device Glyph* items [[buffer(1)]],
                         texture2d<float> atlas [[texture(0)]]) {
    constexpr sampler nearest(coord::normalized, filter::nearest, address::clamp_to_edge);
    Glyph g = items[in.item];
    float2 t = (in.pixel - g.rect.xy) / g.rect.zw;
    float2 uv = mix(g.uv.xy, g.uv.zw, t);
    return g.color * atlas.sample(nearest, uv).r;
}

// --- Paths: stencil, then cover (S6-Q4) ----------------------------------------

struct PathVaryings {
    float4 position [[position]];
};

// Triangle fans of each contour, as a triangle list: each triangle adds its
// winding to the stencil buffer (front faces increment, back faces decrement).
vertex PathVaryings vs_path(uint vid [[vertex_id]],
                            constant Frame& frame [[buffer(0)]],
                            const device float2* vertices [[buffer(1)]]) {
    PathVaryings out;
    out.position = to_clip(vertices[vid], frame);
    return out;
}

fragment float4 fs_path_stencil(PathVaryings in [[stage_in]]) {
    return float4(0.0); // colour writes are masked off; only the stencil changes
}

vertex Varyings vs_cover(uint vid [[vertex_id]], uint iid [[instance_id]],
                         constant Frame& frame [[buffer(0)]],
                         const device Cover* covers [[buffer(1)]]) {
    Cover c = covers[iid];
    float2 pixel = c.rect.xy + corner_of(vid) * c.rect.zw;
    Varyings out;
    out.position = to_clip(pixel, frame);
    out.pixel = pixel;
    out.item = iid;
    return out;
}

fragment float4 fs_cover(Varyings in [[stage_in]], const device Cover* covers [[buffer(1)]]) {
    return covers[in.item].color;
}
