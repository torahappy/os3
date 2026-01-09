#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_color_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> time: f32;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var col = textureSample(base_color_texture, base_color_sampler, mesh.uv);

    if (col.x < 0.3 && col.y > 0.7 && col.z < 0.3) {
        col.w = 0.0;
    } else if (col.x < 0.03 && col.y < 0.03 && col.z < 0.03) {
      col.x = 0.95;
      col.y = 0.01;
      col.z = 0.01;
    } else if (col.x < 0.05 && col.y < 0.05 && col.z < 0.05) {
      col.x = 0.01;
      col.y = 0.99;
      col.z = 0.1;
    }
    if (col.x > 0.99 && col.y > 0.99 && col.z > 0.99) {
      col.x = 0.95;
      col.y = 0.79;
      col.z = 0.89;
    } else if (col.x > 0.9 && col.y > 0.9 && col.z > 0.9) {
      col.x = 0.04;
      col.y = 0.07;
      col.z = 0.08;

    }
    return col;
}
