#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_color_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> time_id_numtxtbox_loss: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // ===== base params =====

    // loss value modification
    var difficulty = 0.5;

    // color value modification for stripes
    var color_mul = 0.5;
    
    // alpha value modification for stripes
    var alpha_mul = 2.0;

    // multiply number of stripes by x times
    var density = 3.0;

    // ===== color calculation =====

    var col = textureSample(base_color_texture, base_color_sampler, mesh.uv);
    var ab_ratio = max(0.0, min(1.0, time_id_numtxtbox_loss.w - difficulty));
    var num_textbox = time_id_numtxtbox_loss.z;
    var my_id = time_id_numtxtbox_loss.y;


    var color_a = vec4(col.r, col.g, col.b, (1.0 - col.r) * (1.0 - ab_ratio));


    var slot_pos = (mesh.uv.x * num_textbox * density % num_textbox) - my_id;
    var color_b = vec4(0.0, 0.0, 0.0, 0.0);

    if 0.0 < slot_pos && slot_pos < 1.0 {

      color_b = vec4(col.r * color_mul, col.g * color_mul, col.b * color_mul, (1.0 - col.r) * ab_ratio * alpha_mul);

    }
    return color_a + color_b;
}
