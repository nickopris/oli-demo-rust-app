use oli_demo_rust_app::indicators::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn sma_of_window() {
    assert_eq!(sma(&[1., 2., 3., 4., 5.], 5), Some(3.0));
    assert_eq!(sma(&[1., 2., 3., 4., 5.], 2), Some(4.5));
    assert_eq!(sma(&[1., 2., 3., 4., 5.], 6), None);
}

#[test]
fn rsi_extremes_and_minimum_length() {
    let up: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
    let down: Vec<f64> = (0..30).map(|i| 100.0 - i as f64).collect();
    assert_eq!(rsi(&up, 14), Some(100.0));
    assert_eq!(rsi(&down, 14), Some(0.0));
    assert_eq!(rsi(&up[..14], 14), None);
}

#[test]
fn macd_flat_series_is_zero() {
    let flat = vec![50.0; 34];
    let m = macd(&flat).expect("34 closes is enough");
    assert!(close(m.line, 0.0) && close(m.signal, 0.0) && close(m.histogram, 0.0));
    assert!(macd(&flat[..33]).is_none());
}

#[test]
fn bollinger_flat_series_collapses() {
    let b = bollinger(&[10.0; 20], 20).unwrap();
    assert!(close(b.upper, b.mid) && close(b.lower, b.mid) && close(b.mid, 10.0));
    assert!(bollinger(&[10.0; 19], 20).is_none());
}

#[test]
fn obv_accumulates_signed_volume() {
    assert_eq!(
        obv(&[1., 2., 2., 1.], &[10., 20., 30., 40.]),
        vec![0., 20., 20., -20.]
    );
    assert!(obv(&[1.], &[1.]).is_empty());
}

#[test]
fn linear_slope_is_normalised_to_mean() {
    assert!(close(linear_slope(&[1., 2., 3.]), 0.5));
    assert_eq!(linear_slope(&[5.]), 0.0);
}

#[test]
fn clamp_bounds() {
    assert_eq!(clamp(150.0, 0.0, 100.0), 100.0);
    assert_eq!(clamp(-1.0, 0.0, 100.0), 0.0);
}
