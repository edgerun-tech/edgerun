use edgerun_wgpu::uniforms::GpuRectStyle;

fn main() {
    let rects = vec![
        GpuRectStyle::solid(0.0, 0.0, 200.0, 200.0, 0x1a, 0x1a, 0x2e, 255, 0),
        GpuRectStyle::solid(50.0, 50.0, 100.0, 100.0, 0xFF, 0x00, 0x00, 255, 1),
    ];

    // Debug: print struct size and offsets
    println!("GpuRectStyle::SIZE = {}", GpuRectStyle::SIZE);
    let data0 = bytemuck::bytes_of(&rects[0]);
    let data1 = bytemuck::bytes_of(&rects[1]);
    println!("rect[0] bg_color bytes: {:02X}{:02X}{:02X}{:02X}", data0[0], data0[1], data0[2], data0[3]);
    println!("rect[1] bg_color bytes: {:02X}{:02X}{:02X}{:02X}", data1[0], data1[1], data1[2], data1[3]);
    println!("rect[0].bg_color f32: [{}, {}, {}, {}]", rects[0].bg_color[0], rects[0].bg_color[1], rects[0].bg_color[2], rects[0].bg_color[3]);
    println!("rect[1].bg_color f32: [{}, {}, {}, {}]", rects[1].bg_color[0], rects[1].bg_color[1], rects[1].bg_color[2], rects[1].bg_color[3]);

    // Check the bytes at offset GpuRectStyle::SIZE
    let all_data: Vec<u8> = rects.iter().flat_map(|r| bytemuck::bytes_of(r).to_vec()).collect();
    println!("Total bytes uploaded: {}", all_data.len());
    println!("Byte at offset 0 (rect[0].bg_color[0]): {:02X}", all_data[0]);
    println!("Byte at offset {} (rect[1].bg_color[0]): {:02X}", GpuRectStyle::SIZE, all_data[GpuRectStyle::SIZE]);

    let text_cmds: Vec<edgerun_wgpu::uniforms::GpuTextCommand> = vec![];
    let pixels = edgerun_wgpu::render::render_to_pixels(200, 200, &rects, &text_cmds, &[], 0.0, "");
    let i = (100 * 200 + 100) as usize * 4;
    println!("GPU pixel (100,100): ({}, {}, {}, {})", pixels[i], pixels[i+1], pixels[i+2], pixels[i+3]);
    let i2 = (10 * 200 + 10) as usize * 4;
    println!("GPU pixel (10,10): ({}, {}, {}, {})", pixels[i2], pixels[i2+1], pixels[i2+2], pixels[i2+3]);
}
