// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use verify_macros::{step, step_module};

pub struct CalculatorWorld {
    display: i32,
}

#[step_module]
pub mod steps {

    use super::*;

    #[step(r"the calculator is clear")]
    async fn clear(w: &mut CalculatorWorld) {
        w.display = 0;
        println!("Display cleared in CalculatorWorld");
    }

    #[step(r"I add (\d+)")]
    async fn add(w: &mut CalculatorWorld, val: i32) {
        w.display += val;
        println!("Added {} to CalculatorWorld", val);
    }

    #[step(r"the display should be (\d+)")]
    async fn check_display(w: &mut CalculatorWorld, expected: i32) {
        assert_eq!(w.display, expected);
        println!("Checked display {} == {}", w.display, expected);
    }
}

impl features::World for CalculatorWorld {}

use features::Features;

pub fn setup_world() -> (Features<CalculatorWorld>, CalculatorWorld) {
    let mut features = Features::<CalculatorWorld>::new();

    steps::register_steps(&mut features);

    let world = CalculatorWorld { display: 0 };
    (features, world)
}

#[tokio::main]
async fn main() {
    let (features, mut world) = setup_world();

    let feature = include_str!("../features/calculator.feat");

    features.execute_from_memory(feature, &mut world).await.unwrap();
}
