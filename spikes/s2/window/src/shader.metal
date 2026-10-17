// Spike S2: draws one textured quad, a text coverage mask, over the cleared
// drawable. Compiled from source at run time (MTLDevice newLibraryWithSource),
// so building the app needs no Xcode `metal` toolchain.
//
// The quad's four corners come from vertex_id (a triangle strip), so there is
// no vertex buffer. Its placement and colour arrive in one `Quad` at
// buffer(0), whose layout must match `platform::gpu::Quad`.

#include <metal_stdlib>
using namespace metal;

struct Quad {
    float2 origin;   // top-left corner, in drawable pixels
    float2 size;     // in drawable pixels
    float2 viewport; // drawable size, in pixels
    float2 padding;  // keeps `color` 16-byte aligned, as on the Rust side
    float4 color;    // straight (not premultiplied) RGBA
};

struct Varyings {
    float4 position [[position]];
    float2 uv;
    float4 color [[flat]];
};

vertex Varyings vs_main(uint vid [[vertex_id]], constant Quad& quad [[buffer(0)]]) {
    float2 corner = float2(float(vid & 1u), float(vid >> 1u)); // (0,0) (1,0) (0,1) (1,1)
    float2 pixel = quad.origin + corner * quad.size;
    float2 ndc = float2(pixel.x / quad.viewport.x * 2.0 - 1.0,
                        1.0 - pixel.y / quad.viewport.y * 2.0);
    Varyings out;
    out.position = float4(ndc, 0.0, 1.0);
    out.uv = corner;
    out.color = quad.color;
    return out;
}

fragment float4 fs_main(Varyings in [[stage_in]], texture2d<float> coverage [[texture(0)]]) {
    constexpr sampler nearest(coord::normalized, filter::nearest, address::clamp_to_edge);
    float alpha = coverage.sample(nearest, in.uv).r * in.color.a;
    return float4(in.color.rgb * alpha, alpha); // premultiplied, for One / OneMinusSourceAlpha blending
}
