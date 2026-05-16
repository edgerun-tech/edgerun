use std::string::String;
use std::vec::Vec;

use super::*;

pub(super) fn push_label(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    text: &str,
    scale: f32,
    color: Color4,
) {
    #[cfg(feature = "fontdue-text")]
    if let Some(atlas) = atlas {
        atlas.layout_text_scaled_into(scene, x, y, text, scale, color);
        return;
    }
    scene.push_text(x, y, text, scale, color);
}

pub(super) fn push_bounded_label(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    max_w: f32,
    text: &str,
    scale: f32,
    color: Color4,
) {
    if max_w <= 0.0 {
        return;
    }
    let label = truncate_label_to_width(
        text,
        max_w,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    );
    push_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x,
        y,
        &label,
        scale,
        color,
    );
}

pub(super) fn draw_pill(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    color: Color4,
) {
    scene.push_rect(GpuRect::fill(x, y, w, 28.0, 14.0, color.with_alpha(0.14)));
    scene.push_rect(GpuRect::border(x, y, w, 28.0, 14.0, color.with_alpha(0.46)));
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 12.0,
        y + 7.0,
        (w - 24.0).max(0.0),
        label,
        2.0,
        color,
    );
}

pub(super) fn draw_contact_row(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    contact: &UnifiedContact<'_>,
    selected: bool,
) {
    soft_card(
        scene,
        x,
        y,
        w,
        56.0,
        10.0,
        if selected {
            palette::ACTIVE_ROW
        } else {
            palette::ROW
        },
    );
    let accent = match contact.kind {
        UnifiedContactKind::Person => palette::ACCENT,
        UnifiedContactKind::CodexClient => palette::VIOLET,
        UnifiedContactKind::Node => palette::GREEN,
    };
    scene.push_rect(GpuRect::fill(
        x + 14.0,
        y + 13.0,
        30.0,
        30.0,
        15.0,
        accent.with_alpha(0.22),
    ));
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 23.0,
        y + 19.0,
        10.0,
        contact_initial(contact.name),
        2.0,
        accent,
    );
    scene.push_rect(GpuRect::fill(
        x + 38.0,
        y + 36.0,
        8.0,
        8.0,
        4.0,
        if contact.online {
            palette::GREEN
        } else {
            palette::MUTED
        },
    ));
    let unread_w = if contact.unread > 0 { 50.0 } else { 0.0 };
    let text_w = (w - 72.0 - unread_w).max(0.0);
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 56.0,
        y + 11.0,
        text_w,
        contact.name,
        2.0,
        if selected {
            palette::TEXT
        } else {
            palette::MUTED
        },
    );
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 56.0,
        y + 32.0,
        text_w,
        contact.detail,
        2.0,
        accent,
    );
    if contact.unread > 0 {
        let label = if contact.unread > 9 { "9+" } else { "new" };
        draw_pill(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            x + w - 58.0,
            y + 14.0,
            42.0,
            label,
            palette::AMBER,
        );
    }
}

pub(super) fn contact_initial(name: &str) -> &str {
    name.get(0..1).unwrap_or("?")
}

pub(super) fn draw_message(
    scene: &mut GpuScene,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
    x: f32,
    y: f32,
    w: f32,
    role: &str,
    body: &str,
    fill: Color4,
    accent: Color4,
) -> f32 {
    let body_w = (w - 42.0).max(120.0);
    let lines = wrap_lines(
        body,
        body_w,
        4,
        #[cfg(feature = "fontdue-text")]
        atlas,
    );
    let h = 56.0 + lines.len() as f32 * 22.0;
    soft_card(scene, x, y, w, h, 14.0, fill);
    scene.push_rect(GpuRect::fill(x, y, 4.0, h, 2.0, accent));
    push_bounded_label(
        scene,
        #[cfg(feature = "fontdue-text")]
        atlas,
        x + 22.0,
        y + 16.0,
        (w - 44.0).max(0.0),
        role,
        2.0,
        accent,
    );
    for (index, line) in lines.iter().enumerate() {
        push_bounded_label(
            scene,
            #[cfg(feature = "fontdue-text")]
            atlas,
            x + 22.0,
            y + 42.0 + index as f32 * 22.0,
            body_w,
            line,
            2.0,
            palette::TEXT,
        );
    }
    h
}

pub(super) fn wrap_lines(
    text: &str,
    max_width: f32,
    max_lines: usize,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> Vec<String> {
    if max_lines == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut consumed_all = true;
    let max_width = max_width.max(1.0);

    'outer: for raw_line in text.split('\n') {
        if raw_line.is_empty() {
            if lines.len() >= max_lines {
                consumed_all = false;
                break;
            }
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in raw_line.split_whitespace() {
            if measure_label_width(
                word,
                2.0,
                #[cfg(feature = "fontdue-text")]
                atlas,
            ) > max_width
            {
                if !current.is_empty() {
                    if lines.len() >= max_lines {
                        consumed_all = false;
                        break 'outer;
                    }
                    lines.push(std::mem::take(&mut current));
                }

                for ch in word.chars() {
                    let had_current = !current.is_empty();
                    current.push(ch);
                    if had_current
                        && measure_label_width(
                            &current,
                            2.0,
                            #[cfg(feature = "fontdue-text")]
                            atlas,
                        ) > max_width
                    {
                        current.pop();
                        if lines.len() >= max_lines {
                            consumed_all = false;
                            break 'outer;
                        }
                        lines.push(std::mem::take(&mut current));
                        current.push(ch);
                    }
                }
                continue;
            }

            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if measure_label_width(
                &candidate,
                2.0,
                #[cfg(feature = "fontdue-text")]
                atlas,
            ) <= max_width
                || current.is_empty()
            {
                current = candidate;
                continue;
            }

            lines.push(current);
            current = word.to_string();

            if lines.len() >= max_lines {
                consumed_all = false;
                break 'outer;
            }
        }

        if lines.len() >= max_lines {
            consumed_all = false;
            break;
        }
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    if !consumed_all {
        if let Some(last) = lines.last_mut() {
            while !last.is_empty()
                && measure_label_width(
                    &format!("{last}..."),
                    2.0,
                    #[cfg(feature = "fontdue-text")]
                    atlas,
                ) > max_width
            {
                last.pop();
            }
            last.push_str("...");
        }
    }

    lines
}

pub(super) fn measure_label_width(
    text: &str,
    scale: f32,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> f32 {
    #[cfg(feature = "fontdue-text")]
    if let Some(atlas) = atlas {
        return atlas.text_width_scaled(text, scale);
    }
    text.chars().count() as f32 * scale.max(1.0) * 6.0
}

pub(super) fn truncate_label_to_width(
    text: &str,
    max_width: f32,
    scale: f32,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> String {
    if measure_label_width(
        text,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    ) <= max_width
    {
        return text.to_string();
    }

    let ellipsis = "...";
    let ellipsis_w = measure_label_width(
        ellipsis,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    );
    if ellipsis_w > max_width {
        return String::new();
    }

    let mut out = String::new();
    for ch in text.chars() {
        out.push(ch);
        let candidate = format!("{out}{ellipsis}");
        if measure_label_width(
            &candidate,
            scale,
            #[cfg(feature = "fontdue-text")]
            atlas,
        ) > max_width
        {
            out.pop();
            break;
        }
    }
    out.push_str(ellipsis);
    out
}

pub(super) fn component_label_width(
    text: &str,
    scale: f32,
    #[cfg(feature = "fontdue-text")] atlas: Option<&FontAtlas>,
) -> f32 {
    measure_label_width(
        text,
        scale,
        #[cfg(feature = "fontdue-text")]
        atlas,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_lines_preserves_explicit_line_breaks() {
        let lines = wrap_lines(
            "first line\nsecond line",
            500.0,
            4,
            #[cfg(feature = "fontdue-text")]
            None,
        );

        assert_eq!(lines, vec!["first line", "second line"]);
    }

    #[test]
    fn wrap_lines_limits_overflow_with_ellipsis() {
        let lines = wrap_lines(
            "one two three four five",
            40.0,
            2,
            #[cfg(feature = "fontdue-text")]
            None,
        );

        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with("..."));
    }

    #[test]
    fn wrap_lines_breaks_unspaced_overflow() {
        let lines = wrap_lines(
            "abcdefgh",
            24.0,
            8,
            #[cfg(feature = "fontdue-text")]
            None,
        );

        assert_eq!(lines, vec!["ab", "cd", "ef", "gh"]);
    }

    #[cfg(feature = "fontdue-text")]
    #[test]
    fn wrap_lines_breaks_long_unbroken_tokens_with_font_atlas() {
        let atlas = FontAtlas::load_geist(18.0).expect("Geist font atlas");
        let lines = wrap_lines(
            "blake3policyhashwithoutseparators",
            atlas.text_width("blake3p") + 1.0,
            8,
            Some(&atlas),
        );

        assert!(lines.len() > 1);
        assert!(
            lines
                .iter()
                .all(|line| atlas.text_width(line) <= atlas.text_width("blake3p") + 1.0)
        );
    }

    #[test]
    fn truncate_label_to_width_adds_single_line_ellipsis() {
        let label = truncate_label_to_width(
            "policy-hash-blake3-route-admission",
            84.0,
            2.0,
            #[cfg(feature = "fontdue-text")]
            None,
        );

        assert!(label.ends_with("..."));
        assert!(label.len() < "policy-hash-blake3-route-admission".len());
        assert!(
            measure_label_width(
                &label,
                2.0,
                #[cfg(feature = "fontdue-text")]
                None,
            ) <= 84.0
        );
    }

    #[test]
    fn truncate_label_to_width_returns_empty_when_even_ellipsis_cannot_fit() {
        let label = truncate_label_to_width(
            "abc",
            4.0,
            2.0,
            #[cfg(feature = "fontdue-text")]
            None,
        );

        assert_eq!(label, "");
    }

    #[test]
    fn operational_tokens_fit_or_truncate_deterministically() {
        let samples = [
            "manifest blake3:9bf4076a91d0cc2e1f847cc8f8ce11af",
            "policy:publisher:mail:7f2c35aa01b9d0f7",
            "route:admission->relay:private-devices->storage:vps-cache",
            "184 units reserved",
            "identity:edgerun:3f0d2a8b9c114e71",
        ];

        for sample in samples {
            let label = truncate_label_to_width(
                sample,
                132.0,
                2.0,
                #[cfg(feature = "fontdue-text")]
                None,
            );
            assert!(
                measure_label_width(
                    &label,
                    2.0,
                    #[cfg(feature = "fontdue-text")]
                    None,
                ) <= 132.0,
                "{sample} produced oversized label {label}"
            );
            if label != sample {
                assert!(
                    label.ends_with("..."),
                    "{sample} truncated without ellipsis"
                );
            }
        }
    }

    #[cfg(feature = "fontdue-text")]
    #[test]
    fn truncate_label_to_width_uses_font_atlas_measurements() {
        let atlas = FontAtlas::load_geist(18.0).expect("Geist font atlas");
        let label = truncate_label_to_width(
            "publisher-policy-hash-blake3",
            atlas.text_width("publisher-policy") + 18.0,
            2.0,
            Some(&atlas),
        );

        assert!(label.ends_with("..."));
        assert!(atlas.text_width(&label) <= atlas.text_width("publisher-policy") + 18.0);
    }
}

pub(super) fn soft_card(
    scene: &mut GpuScene,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: Color4,
) {
    scene.push_rect(GpuRect::shadow(
        x,
        y + 10.0,
        w,
        h,
        radius,
        Color4::rgba(0.0, 0.0, 0.0, 0.22),
        24.0,
    ));
    scene.push_rect(GpuRect::fill(x, y, w, h, radius, color));
}

pub(super) fn panel(
    scene: &mut GpuScene,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: Color4,
) {
    scene.push_rect(GpuRect::fill(x, y, w, h, radius, color));
    scene.push_rect(GpuRect::border(x, y, w, h, radius, palette::BORDER));
}
