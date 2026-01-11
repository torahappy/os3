#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_color_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> time: f32;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> category: f32;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> level: f32;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var col = textureSample(base_color_texture, base_color_sampler, mesh.uv);

    if (col.w > 0.99) {
      col.w = 0.4;
    } else {
      col.w *= 0.07;
    }

    if (0.99 < level && level < 1.01) {
      col.x += 0.2;
      col.y += 0.2;
      col.w += 0.1;
    }

    if (category > 1.99) {
	col.x = 1.0 - col.x;
	col.y = 1.0 - col.y;
	col.z = 1.0 - col.z;
	col.z *= 0.5;
      }
    if (level < 0.1) {
	col.w = 0.0;
      }

    return col;
}
