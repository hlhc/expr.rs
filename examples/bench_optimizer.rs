//! Benchmark: optimized vs unoptimized AST evaluation.
//!
//! Compiles the same compound expression with and without the optimizer,
//! verifies correctness, then benchmarks evaluation speed.
//!
//! Run with: `cargo run --example bench_optimizer --release`

use expr::{Context, Environment, compile_opts};
use std::time::Instant;

fn main() {
    let code = r#"
        let x = 2 + 3;
        let unused = 999;
        let y = x * 1 + 0;
        let flag = true && false || true;
        let choice = true ? y : 0;
        let r = 0..2;
        let c = [1, 2] | len();
        let info = {foo: "bar"}.foo;
        let v = 5 ?? 10;
        let idx = r[1];
        choice + idx
    "#;

    let iters: u64 = 500_000;
    let env = Environment::new();
    let ctx = Context::default();

    let t0 = Instant::now();
    let prog_opt = compile_opts(code, true).unwrap();
    let compile_opt_us = t0.elapsed().as_micros();

    let t0 = Instant::now();
    let prog_raw = compile_opts(code, false).unwrap();
    let compile_raw_us = t0.elapsed().as_micros();

    let nodes_opt = prog_opt.node_count();
    let nodes_raw = prog_raw.node_count();
    let reduction = 100.0 - (nodes_opt as f64 / nodes_raw as f64 * 100.0);

    println!("\n  Optimized AST:");
    println!("  Lines ({}):", prog_opt.lines().len());
    for (id, node) in prog_opt.lines() {
        println!("    let {} = {:?}", id, node);
    }
    println!("  Expr: {:?}", prog_opt.expr());
    println!("  Total nodes: {}", prog_opt.node_count());

    let result = env.run(prog_opt.clone(), &ctx).unwrap();
    assert_eq!(result, env.run(prog_raw.clone(), &ctx).unwrap());

    // warm-up
    for _ in 0..1000 {
        let _ = env.run(prog_opt.clone(), &ctx);
        let _ = env.run(prog_raw.clone(), &ctx);
    }

    let t0 = Instant::now();
    for _ in 0..iters {
        let _ = env.run(prog_opt.clone(), &ctx);
    }
    let opt_eval_us = t0.elapsed().as_micros();
    let opt_ns = opt_eval_us * 1_000 / iters as u128;

    let t0 = Instant::now();
    for _ in 0..iters {
        let _ = env.run(prog_raw.clone(), &ctx);
    }
    let raw_eval_us = t0.elapsed().as_micros();
    let raw_ns = raw_eval_us * 1_000 / iters as u128;

    let opt_total_us = compile_opt_us as f64 + opt_eval_us as f64 / iters as f64;
    let raw_total_us = compile_raw_us as f64 + raw_eval_us as f64 / iters as f64;

    let speedup = raw_ns as f64 / opt_ns as f64;
    let compile_ratio = compile_raw_us as f64 / compile_opt_us as f64;
    let (compile_dir, compile_mult) = if compile_opt_us <= compile_raw_us {
        ("faster", compile_ratio)
    } else {
        ("slower", compile_opt_us as f64 / compile_raw_us as f64)
    };
    let (eval_dir, eval_mult) = if opt_ns <= raw_ns {
        ("faster", speedup)
    } else {
        ("slower", raw_ns as f64 / opt_ns as f64)
    };
    let (total_dir, total_ratio) = if opt_total_us <= raw_total_us {
        ("faster", raw_total_us / opt_total_us)
    } else {
        ("slower", opt_total_us / raw_total_us)
    };


    println!("
  expr-lang Optimizer Benchmark
  ─────────────────────────────

  Expression:
");
    for line in code.lines() {
        let t = line.trim();
        if !t.is_empty() {
            println!("    {}", t);
        }
    }

    println!(
        "
  Result:  {}
  Nodes:   {} → {}  ({:.0}% smaller)
  Compile: {} µs → {} µs  ({:.1}x {})
  Eval:    {} ns → {} ns  ({:.1}x {})
  Total:   {:.1} µs → {:.1} µs  ({:.1}x {})
",
        result,
        nodes_raw, nodes_opt, reduction,
        compile_raw_us, compile_opt_us, compile_mult, compile_dir,
        raw_ns, opt_ns, eval_mult, eval_dir,
        raw_total_us, opt_total_us, total_ratio, total_dir,
    );
}
