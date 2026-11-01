use indium::runtime::scheduler::FixedStep;
#[test]
fn ticks_do_not_depend_on_frame_rate() {
    for fps in [30, 60, 144, 240, 360] {
        let mut clock = FixedStep::default();
        let mut ticks = 0;
        for _ in 0..fps {
            let (count, dropped) = clock.advance(1.0 / fps as f64, 240.0, 16);
            ticks += count;
            assert_eq!(dropped, 0);
        }
        assert_eq!(ticks, 240, "at {fps} FPS");
    }
}
#[test]
fn overload_is_bounded_and_backlog_is_discarded() {
    let mut clock = FixedStep::default();
    assert_eq!(clock.advance(1.0, 240.0, 8), (8, 232));
    assert_eq!(clock.advance(1.0 / 240.0, 240.0, 8), (1, 0));
}
#[test]
fn partial_ticks_accumulate() {
    let mut clock = FixedStep::default();
    assert_eq!(clock.advance(0.002, 240.0, 8), (0, 0));
    assert_eq!(clock.advance(0.003, 240.0, 8), (1, 0));
    assert!((clock.accumulated - 0.000833333333333333).abs() < 1e-9);
}
