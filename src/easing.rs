/// see: https://dhemery.github.io/DHE-Modules/technical/sigmoid/
/// TL;DR: given normalised values (i.e. -1 to 1), produces an easing function
/// change `k` to change the easing:
///   - `k` == 0 is linear
///   - `k` with larger positive values results in a sharper ease out followed by a sharper ease in
///   - `k` with larger negative values results in a sharper ease in followed by a sharper ease out
pub fn sigmoid(k: f32) -> Box<dyn Fn(f32) -> f32> {
    Box::new(move |x: f32| (x - (k * x)) / (k - (2.0 * k * x.abs()) + 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(f: &dyn Fn(f32) -> f32) -> Vec<(f32, f32)> {
        [
            -1.0, -0.9, -0.75, -0.5, -0.25, 0.0, 0.1, 0.25, 0.45, 0.5, 0.55, 0.75, 0.9, 1.0,
        ]
        .into_iter()
        .map(|x| (x, f(x)))
        .collect::<Vec<_>>()
    }

    #[test]
    fn k_0_is_linear() {
        let samples = sample(&sigmoid(0.0));

        assert_eq!(
            samples,
            vec![
                (-1.0, -1.0),
                (-0.9, -0.9),
                (-0.75, -0.75),
                (-0.5, -0.5),
                (-0.25, -0.25),
                (0.0, 0.0),
                (0.1, 0.1),
                (0.25, 0.25),
                (0.45, 0.45),
                (0.5, 0.5),
                (0.55, 0.55),
                (0.75, 0.75),
                (0.9, 0.9),
                (1.0, 1.0),
            ]
        );
    }

    #[test]
    fn large_positive_k_has_sharp_ease_out_and_ease_in() {
        let samples = sample(&sigmoid(0.95));

        assert_eq!(
            samples,
            vec![
                (-1.0, -1.0),
                (-0.9, -0.18750001),
                (-0.75, -0.07142861),
                (-0.5, -0.025000006),
                (-0.25, -0.008474578),
                (0.0, 0.0),
                (0.1, 0.0028409106),
                (0.25, 0.008474578),
                (0.45, 0.020547953),
                (0.5, 0.025000006),
                (0.55, 0.030386776),
                (0.75, 0.07142861),
                (0.9, 0.18750001),
                (1.0, 1.0),
            ]
        );
    }

    #[test]
    fn small_positive_k_has_smooth_ease_out_and_ease_in() {
        let samples = sample(&sigmoid(0.1));

        assert_eq!(
            samples,
            vec![
                (-1.0, -1.0),
                (-0.9, -0.88043475),
                (-0.75, -0.71052635),
                (-0.5, -0.45),
                (-0.25, -0.21428572),
                (0.0, 0.0),
                (0.1, 0.083333336),
                (0.25, 0.21428572),
                (0.45, 0.4009901),
                (0.5, 0.45),
                (0.55, 0.5),
                (0.75, 0.71052635),
                (0.9, 0.88043475),
                (1.0, 1.0),
            ]
        );
    }

    #[test]
    fn large_negative_k_has_sharp_ease_in_and_ease_out() {
        let samples = sample(&sigmoid(-0.95));

        assert_eq!(
            samples,
            vec![
                (-1.0, -1.0),
                (-0.9, -0.997159),
                (-0.75, -0.9915255),
                (-0.5, -0.975),
                (-0.25, -0.9285715),
                (0.0, 0.0),
                (0.1, 0.81249994),
                (0.25, 0.9285715),
                (0.45, 0.9696132),
                (0.5, 0.975),
                (0.55, 0.979452),
                (0.75, 0.9915255),
                (0.9, 0.997159),
                (1.0, 1.0),
            ]
        );
    }

    #[test]
    fn small_negative_k_has_smooth_ease_in_and_ease_out() {
        let samples = sample(&sigmoid(-0.1));

        assert_eq!(
            samples,
            vec![
                (-1.0, -1.0),
                (-0.9, -0.91666657),
                (-0.75, -0.7857143),
                (-0.5, -0.55),
                (-0.25, -0.28947368),
                (0.0, 0.0),
                (0.1, 0.11956521),
                (0.25, 0.28947368),
                (0.45, 0.49999997),
                (0.5, 0.55),
                (0.55, 0.59900993),
                (0.75, 0.7857143),
                (0.9, 0.91666657),
                (1.0, 1.0),
            ]
        );
    }
}
