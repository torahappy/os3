#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_color_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> time: f32;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var col = textureSample(base_color_texture, base_color_sampler, mesh.uv);

    if (col.x > 0.99 && col.y > 0.99 && col.z > 0.99) {
      col.w = 0.03;
    }
    col.x = 1.0 - col.x;
    col.y = 1.0 - col.y;
    col.z = 1.0 - col.z;
    return col;
}
