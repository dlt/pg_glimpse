const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub fn render_sparkline(data: &[u64], width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    // Take last `width` values, pad with 0 if fewer
    let start = data.len().saturating_sub(width);
    let window: Vec<u64> = if data.len() >= width {
        data[start..].to_vec()
    } else {
        let mut v = vec![0u64; width - data.len()];
        v.extend_from_slice(data);
        v
    };

    let min = window.iter().copied().min().unwrap_or(0);
    let max = window.iter().copied().max().unwrap_or(0);

    window
        .iter()
        .map(|&v| {
            if max == min {
                if v == 0 { BLOCKS[0] } else { BLOCKS[4] }
            } else {
                let idx = ((v - min) as f64 / (max - min) as f64 * 7.0).round() as usize + 1;
                BLOCKS[idx.min(8)]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_zeros() {
        let s = render_sparkline(&[0, 0, 0], 5);
        assert_eq!(s.chars().count(), 5);
        assert!(s.chars().all(|c| c == ' '));
    }

    #[test]
    fn constant_nonzero() {
        let s = render_sparkline(&[5, 5, 5], 3);
        assert_eq!(s.chars().count(), 3);
        assert!(s.chars().all(|c| c == '▄'));
    }

    #[test]
    fn ascending() {
        let data: Vec<u64> = (0..10).collect();
        let s = render_sparkline(&data, 10);
        assert_eq!(s.chars().count(), 10);
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars[0], '▁');
        assert_eq!(chars[9], '█');
    }

    #[test]
    fn padding() {
        let s = render_sparkline(&[10], 5);
        assert_eq!(s.chars().count(), 5);
    }

    #[test]
    fn empty_width() {
        let s = render_sparkline(&[1, 2, 3], 0);
        assert!(s.is_empty());
    }
}
