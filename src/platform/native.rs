use crate::engine::input::InputState;
use crate::engine::render::DrawList;
use crate::game::{Game, MatchState, Side};
use std::time::Instant;

/// Headless native runner: steps the fixed simulation a number of ticks and
/// prints a summary. Useful as a smoke test / sanity check that the engine
/// runs and is deterministic. No rendering happens here.
pub fn run_native(args: &[String]) {
    let bench: Option<u64> = args
        .iter()
        .position(|a| a == "--bench")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok());

    let tick_rate: u64 = args
        .iter()
        .position(|a| a == "--ticks")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);

    let dt = 1.0 / 60.0;
    let mut game = Game::new();
    let mut input = InputState::new();
    let mut list = DrawList::new();

    if let Some(iters) = bench {
        let (min, avg, p95, p99, max) = run_benchmark(&mut game, &mut input, &mut list, dt, iters);
        println!(
            "cube_combat bench: {iters} ticks (sim + draw)  min {min:.2}us  avg {avg:.2}us  p95 {p95:.2}us  p99 {p99:.2}us  max {max:.2}us"
        );
        if p95 > 16_667.0 {
            println!("WARN: p95 exceeds the 16.7ms (60 fps) frame budget");
        }
        return;
    }

    println!(
        "cube_combat native sim: {} ticks @ {} Hz ({} simulated seconds)",
        tick_rate,
        1.0 / dt,
        tick_rate as f64 * dt as f64
    );

    let started = Instant::now();
    for _ in 0..tick_rate {
        let frame = input.begin_frame();
        game.simulate(dt, &input, &frame);
    }
    let elapsed = started.elapsed();

    let wins_b = game.wins(Side::Blue);
    let wins_r = game.wins(Side::Red);
    let status = match game.state() {
        MatchState::Playing => "playing",
        MatchState::Ended(Side::Blue) => "blue won",
        MatchState::Ended(Side::Red) => "red won",
    };
    println!(
        "result: {status}  (blue {wins_b}-{wins_r} red)  wall time: {:.1} ms",
        elapsed.as_secs_f64() * 1000.0
    );

    // Draw into a list to exercise the full command pipeline.
    game.draw(&mut list);
}

fn run_benchmark(
    game: &mut Game,
    input: &mut InputState,
    list: &mut DrawList,
    dt: f32,
    iters: u64,
) -> (f64, f64, f64, f64, f64) {
    let mut samples = Vec::with_capacity(iters as usize);
    for _ in 0..iters {
        let frame = input.begin_frame();
        let t0 = Instant::now();
        game.simulate(dt, input, &frame);
        game.draw(list);
        samples.push(t0.elapsed().as_secs_f64() * 1e6);
    }

    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = samples.len();
    let pct = |p: f64| samples[((n as f64 * p) as usize).min(n - 1)];
    (
        samples[0],
        samples.iter().sum::<f64>() / n as f64,
        pct(0.95),
        pct(0.99),
        samples[n - 1],
    )
}