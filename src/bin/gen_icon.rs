use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn dist_to_segment(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (vx, vy) = (b[0] - a[0], b[1] - a[1]);
    let (wx, wy) = (p[0] - a[0], p[1] - a[1]);
    let c1 = wx * vx + wy * vy;
    if c1 <= 0.0 {
        return ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt();
    }
    let c2 = vx * vx + vy * vy;
    if c2 <= c1 {
        return ((p[0] - b[0]).powi(2) + (p[1] - b[1]).powi(2)).sqrt();
    }
    let b_factor = c1 / c2;
    let (proj_x, proj_y) = (a[0] + b_factor * vx, a[1] + b_factor * vy);
    ((p[0] - proj_x).powi(2) + (p[1] - proj_y).powi(2)).sqrt()
}

fn min_polyline_dist(p: [f32; 2], pts: &[[f32; 2]; 5]) -> f32 {
    pts.windows(2)
        .map(|w| dist_to_segment(p, w[0], w[1]))
        .fold(f32::MAX, f32::min)
}

fn main() {
    let size: usize = 1024;
    let mut pixels = vec![0u8; size * size * 4];
    let (center, radius, corner_r, stroke_radius) = (512.0f32, 440.0f32, 190.0f32, 26.0f32);
    let m_points = [
        [240.0, 730.0],
        [385.0, 280.0],
        [512.0, 560.0],
        [639.0, 280.0],
        [784.0, 730.0],
    ];

    for y in 0..size {
        let py = y as f32;
        for x in 0..size {
            let px = x as f32;
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
                let mut r = 10.0 * (1.0 - t);
                let mut g = 132.0 * (1.0 - t) + 89.0 * t;
                let mut b = 255.0 * (1.0 - t) + 214.0 * t;

                if d > -8.0 && py < center {
                    let rim = ((-d) / 8.0).clamp(0.0, 1.0);
                    r = r * rim + 60.0 * (1.0 - rim);
                    g = g * rim + 175.0 * (1.0 - rim);
                    b = b * rim + 255.0 * (1.0 - rim);
                }

                let min_shadow_d = min_polyline_dist([px, py - 18.0], &m_points);
                if min_shadow_d < stroke_radius + 24.0 {
                    let sf = 1.0
                        - (((stroke_radius + 24.0 - min_shadow_d) / 24.0).clamp(0.0, 1.0) * 0.35);
                    r *= sf;
                    g *= sf;
                    b *= sf;
                }

                let min_m_d = min_polyline_dist([px, py], &m_points);
                if min_m_d <= stroke_radius + 2.0 {
                    let am = ((stroke_radius + 1.5 - min_m_d) / 2.0).clamp(0.0, 1.0);
                    r = r * (1.0 - am) + 255.0 * am;
                    g = g * (1.0 - am) + 255.0 * am;
                    b = b * (1.0 - am) + 255.0 * am;
                }

                pixels[idx..idx + 4].copy_from_slice(&[
                    b.round() as u8,
                    g.round() as u8,
                    r.round() as u8,
                    255,
                ]);
            } else if d <= 2.5 {
                let alpha = ((2.5 - d) / 2.5).clamp(0.0, 1.0);
                pixels[idx..idx + 4].copy_from_slice(&[220, 95, 0, (alpha * 255.0).round() as u8]);
            }
        }
    }

    let out_dir = Path::new("assets/icons");
    std::fs::create_dir_all(out_dir).unwrap();
    let tga_path = out_dir.join("icon_1024.tga");
    let mut file = File::create(&tga_path).unwrap();
    let mut header = [0u8; 18];
    header[2] = 2;
    header[12] = (size & 0xFF) as u8;
    header[13] = ((size >> 8) & 0xFF) as u8;
    header[14] = (size & 0xFF) as u8;
    header[15] = ((size >> 8) & 0xFF) as u8;
    header[16] = 32;
    header[17] = 0x28;
    file.write_all(&header).unwrap();
    file.write_all(&pixels).unwrap();

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
    std::fs::remove_file(tga_path).unwrap();

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
