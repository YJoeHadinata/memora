use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn dist_to_segment(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let vx = b[0] - a[0];
    let vy = b[1] - a[1];
    let wx = p[0] - a[0];
    let wy = p[1] - a[1];

    let c1 = wx * vx + wy * vy;
    if c1 <= 0.0 {
        return ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt();
    }

    let c2 = vx * vx + vy * vy;
    if c2 <= c1 {
        return ((p[0] - b[0]).powi(2) + (p[1] - b[1]).powi(2)).sqrt();
    }

    let b_factor = c1 / c2;
    let proj_x = a[0] + b_factor * vx;
    let proj_y = a[1] + b_factor * vy;
    ((p[0] - proj_x).powi(2) + (p[1] - proj_y).powi(2)).sqrt()
}

fn main() {
    let size: usize = 1024;
    let mut pixels = vec![0u8; size * size * 4]; // BGRA for 32-bit TGA

    let center = size as f32 / 2.0;
    let radius = 440.0;
    let corner_r = 190.0;

    // Memora Waveform 'M' vertices (in 1024x1024 canvas)
    let m_points = [
        [240.0, 730.0],
        [385.0, 280.0],
        [512.0, 560.0],
        [639.0, 280.0],
        [784.0, 730.0],
    ];
    let stroke_radius = 26.0;

    for y in 0..size {
        let py = y as f32;
        for x in 0..size {
            let px = x as f32;

            // Distance to Apple squircle contour
            let dx = (px - center).abs() - (radius - corner_r);
            let dy = (py - center).abs() - (radius - corner_r);

            let d = if dx > 0.0 && dy > 0.0 {
                (dx * dx + dy * dy).sqrt() - corner_r
            } else {
                dx.max(dy) - corner_r
            };

            let idx = (y * size + x) * 4;

            if d <= 0.0 {
                let t = py / size as f32;
                // Apple Blue Gradient: #0A84FF (top) to #0059D6 (bottom)
                let mut r = 10.0 * (1.0 - t) + 0.0 * t;
                let mut g = 132.0 * (1.0 - t) + 89.0 * t;
                let mut b = 255.0 * (1.0 - t) + 214.0 * t;

                // Subtle inner top rim light
                if d > -8.0 && py < center {
                    let rim_alpha = ((-d) / 8.0).clamp(0.0, 1.0);
                    r = r * rim_alpha + 60.0 * (1.0 - rim_alpha);
                    g = g * rim_alpha + 175.0 * (1.0 - rim_alpha);
                    b = b * rim_alpha + 255.0 * (1.0 - rim_alpha);
                }

                // Ambient drop shadow for the 'M' symbol (offset by +18y)
                let p_shadow = [px, py - 18.0];
                let d_s1 = dist_to_segment(p_shadow, m_points[0], m_points[1]);
                let d_s2 = dist_to_segment(p_shadow, m_points[1], m_points[2]);
                let d_s3 = dist_to_segment(p_shadow, m_points[2], m_points[3]);
                let d_s4 = dist_to_segment(p_shadow, m_points[3], m_points[4]);
                let min_shadow_d = d_s1.min(d_s2).min(d_s3).min(d_s4);

                if min_shadow_d < stroke_radius + 24.0 {
                    let shadow_t = ((stroke_radius + 24.0 - min_shadow_d) / 24.0).clamp(0.0, 1.0);
                    let shadow_factor = 1.0 - (shadow_t * 0.35);
                    r *= shadow_factor;
                    g *= shadow_factor;
                    b *= shadow_factor;
                }

                // Distance to Memora Waveform 'M'
                let p = [px, py];
                let d1 = dist_to_segment(p, m_points[0], m_points[1]);
                let d2 = dist_to_segment(p, m_points[1], m_points[2]);
                let d3 = dist_to_segment(p, m_points[2], m_points[3]);
                let d4 = dist_to_segment(p, m_points[3], m_points[4]);
                let min_m_d = d1.min(d2).min(d3).min(d4);

                if min_m_d <= stroke_radius + 2.0 {
                    // Crisp antialiased white 'M' symbol
                    let alpha_m = ((stroke_radius + 1.5 - min_m_d) / 2.0).clamp(0.0, 1.0);
                    r = r * (1.0 - alpha_m) + 255.0 * alpha_m;
                    g = g * (1.0 - alpha_m) + 255.0 * alpha_m;
                    b = b * (1.0 - alpha_m) + 255.0 * alpha_m;
                }

                pixels[idx] = b.round() as u8;
                pixels[idx + 1] = g.round() as u8;
                pixels[idx + 2] = r.round() as u8;
                pixels[idx + 3] = 255;
            } else if d <= 2.5 {
                // Antialiasing for outer squircle boundary
                let alpha = ((2.5 - d) / 2.5).clamp(0.0, 1.0);
                pixels[idx] = 220;
                pixels[idx + 1] = 95;
                pixels[idx + 2] = 0;
                pixels[idx + 3] = (alpha * 255.0).round() as u8;
            } else {
                pixels[idx] = 0;
                pixels[idx + 1] = 0;
                pixels[idx + 2] = 0;
                pixels[idx + 3] = 0;
            }
        }
    }

    let out_dir = Path::new("assets/icons");
    std::fs::create_dir_all(out_dir).unwrap();

    let tga_path = out_dir.join("icon_1024.tga");
    let mut file = File::create(&tga_path).unwrap();

    // 18-byte TGA Header (uncompressed true-color BGRA, top-left origin)
    let mut header = [0u8; 18];
    header[2] = 2; // Uncompressed true-color
    header[12] = (size & 0xFF) as u8;
    header[13] = ((size >> 8) & 0xFF) as u8;
    header[14] = (size & 0xFF) as u8;
    header[15] = ((size >> 8) & 0xFF) as u8;
    header[16] = 32; // 32 bits per pixel (BGRA)
    header[17] = 0x28; // Top-left origin, 8 bits alpha

    file.write_all(&header).unwrap();
    file.write_all(&pixels).unwrap();
    println!("Wrote TGA: {:?}", tga_path);

    // Convert TGA to PNG using macOS sips
    let png_path = out_dir.join("icon_1024.png");
    Command::new("sips")
        .args([
            "-s",
            "format",
            "png",
            tga_path.to_str().unwrap(),
            "--out",
            png_path.to_str().unwrap(),
        ])
        .output()
        .expect("sips failed to convert to png");

    println!("Converted to PNG: {:?}", png_path);
    std::fs::remove_file(tga_path).unwrap();

    // Create AppIcon.iconset
    let iconset_dir = out_dir.join("AppIcon.iconset");
    std::fs::create_dir_all(&iconset_dir).unwrap();

    let sizes = [
        (16, "icon_16x16.png"),
        (32, "icon_16x16@2x.png"),
        (32, "icon_32x32.png"),
        (64, "icon_32x32@2x.png"),
        (128, "icon_128x128.png"),
        (256, "icon_128x128@2x.png"),
        (256, "icon_256x256.png"),
        (512, "icon_256x256@2x.png"),
        (512, "icon_512x512.png"),
        (1024, "icon_512x512@2x.png"),
    ];

    for (s, filename) in sizes {
        let dest = iconset_dir.join(filename);
        let s_str = s.to_string();
        Command::new("sips")
            .args([
                "-z",
                &s_str,
                &s_str,
                png_path.to_str().unwrap(),
                "--out",
                dest.to_str().unwrap(),
            ])
            .output()
            .expect("sips resize failed");
    }

    // Convert iconset to .icns using iconutil
    let icns_path = out_dir.join("AppIcon.icns");
    Command::new("iconutil")
        .args([
            "-c",
            "icns",
            iconset_dir.to_str().unwrap(),
            "-o",
            icns_path.to_str().unwrap(),
        ])
        .output()
        .expect("iconutil failed");

    println!("Successfully built AppIcon.icns at {:?}", icns_path);
    std::fs::copy(
        iconset_dir.join("icon_256x256.png"),
        out_dir.join("icon_256.png"),
    )
    .unwrap();
    std::fs::copy(
        iconset_dir.join("icon_512x512.png"),
        out_dir.join("icon_512.png"),
    )
    .unwrap();
    std::fs::remove_dir_all(iconset_dir).unwrap();
    std::fs::remove_file(png_path).unwrap();
}
