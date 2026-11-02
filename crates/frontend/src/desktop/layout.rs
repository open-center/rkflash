//! 由内容宽度驱动的响应式布局策略。

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LayoutProfile {
    pub split_content: bool,
    pub compact_header: bool,
    pub content_margin: i8,
    pub card_gap: f32,
}

impl LayoutProfile {
    const SPLIT_CONTENT_MIN_WIDTH: f32 = 1_180.0;
    const COMPACT_HEADER_MAX_WIDTH: f32 = 1_040.0;

    pub fn from_width(width: f32) -> Self {
        Self {
            split_content: width >= Self::SPLIT_CONTENT_MIN_WIDTH,
            compact_header: width <= Self::COMPACT_HEADER_MAX_WIDTH,
            content_margin: if width <= Self::COMPACT_HEADER_MAX_WIDTH {
                16
            } else {
                22
            },
            card_gap: 12.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LayoutProfile;

    #[test]
    fn changes_structure_at_content_driven_widths() {
        let minimum = LayoutProfile::from_width(960.0);
        assert!(minimum.compact_header);
        assert!(!minimum.split_content);
        assert_eq!(minimum.content_margin, 16);

        let intermediate = LayoutProfile::from_width(1_080.0);
        assert!(!intermediate.compact_header);
        assert!(!intermediate.split_content);

        let wide = LayoutProfile::from_width(1_320.0);
        assert!(!wide.compact_header);
        assert!(wide.split_content);
        assert_eq!(wide.content_margin, 22);
    }
}
