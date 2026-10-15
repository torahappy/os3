#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_color_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> time_id_numtxtbox_loss: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var col = textureSample(base_color_texture, base_color_sampler, mesh.uv);
    var alpha = max(0.0, min(1.0, time_id_numtxtbox_loss.w));
    var num_textbox = time_id_numtxtbox_loss.z;
    var my_id = time_id_numtxtbox_loss.y;


    var ab_ratio = time_id_numtxtbox_loss.w;

    var color_a = vec4(col.r, col.g, col.b, (1.0 - col.r) * (1.0 - ab_ratio));

    var density = 3.0;

    var slot_pos = (mesh.uv.x * num_textbox * density % num_textbox) - my_id;
    var color_b = vec4(0.0, 0.0, 0.0, 0.0);

    if 0.0 < slot_pos && slot_pos < 1.0 {

      color_b = vec4(col.r, col.g, col.b, (1.0 - col.r) * ab_ratio);

    }
    return color_a + color_b;
}
