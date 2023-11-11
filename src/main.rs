#![feature(slice_ptr_len)]
#![feature(raw_slice_split)]

use std::time::Instant;

mod chunk_iter;
mod physics;
mod shader;

use miniquad::*;

use glam::{vec2, Mat4, Vec2};
use physics::{FlagParams, Physics};

const SUB_STEPS: usize = 10;
const WIDTH: usize = 1500;
const HEIGHT: usize = 1500;

enum UpdateCommand {
    OneFrame,
    Continue,
    Stop,
    Quit,
}

struct Stage {
    pipeline: Pipeline,
    bindings: Bindings,

    physics: Physics,
    last_frame: Instant,
    frame_count: usize,
    mouse_pressed: bool,
    mouse_pos: Vec2,
    last_mouse_pos: Vec2,
    can_update: UpdateCommand,
    accumulate_time: u128,
}

impl Stage {
    pub fn new(ctx: &mut Context) -> Stage {
        quad_rand::srand(1);

        let physics = Physics::new(&[FlagParams {
            corner: vec2(100.0, 100.0),
            size: 1000.0,
            width: 50,
            height: 30,
        }]);

        let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &physics.get_indices());

        let positions_vertex_buffer = Buffer::stream(
            ctx,
            BufferType::VertexBuffer,
            physics.get_points().len() * std::mem::size_of::<Vec2>(),
        );

        let bindings = Bindings {
            vertex_buffers: vec![positions_vertex_buffer],
            index_buffer,
            images: vec![],
        };

        let shader = Shader::new(ctx, shader::VERTEX, shader::FRAGMENT, shader::meta()).unwrap();

        let pipeline = Pipeline::with_params(
            ctx,
            &[BufferLayout::default()],
            &[VertexAttribute::new("pos", VertexFormat::Float2)],
            shader,
            PipelineParams {
                primitive_type: PrimitiveType::Lines,
                ..Default::default()
            },
        );

        Stage {
            pipeline,
            bindings,
            physics,
            last_frame: Instant::now(),
            frame_count: 0,
            mouse_pressed: false,
            mouse_pos: Vec2::ZERO,
            last_mouse_pos: Vec2::ZERO,
            can_update: UpdateCommand::Continue,
            accumulate_time: 0,
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self, ctx: &mut Context) {
        match self.can_update {
            UpdateCommand::Stop => return,
            UpdateCommand::Quit => {
                ctx.quit();
                return;
            }
            _ => (),
        }

        let start = Instant::now();
        let dt = 1. / 60.;

        // update particle positions
        for _ in 0..SUB_STEPS {
            self.physics.step(vec2(200.0, 50.0), dt / SUB_STEPS as f32);
        }

        if self.mouse_pressed {
            self.physics.move_selected_nodes(self.mouse_pos);
        }

        self.frame_count += 1;
        self.accumulate_time += self.last_frame.elapsed().as_micros();
        if self.frame_count % 120 == 0 {
            println!(
                "fps: {}, time to update: {}",
                1000000 / (self.accumulate_time / 120),
                start.elapsed().as_micros()
            );
            self.accumulate_time = 0;
        }
        self.last_frame = Instant::now();

        if let UpdateCommand::OneFrame = self.can_update {
            self.can_update = UpdateCommand::Stop;
        }
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32) {
        self.last_mouse_pos = self.mouse_pos;
        self.mouse_pos = vec2(x, y);
    }

    fn mouse_button_down_event(&mut self, _: &mut Context, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            self.last_mouse_pos = self.mouse_pos;
            self.mouse_pos = vec2(x, y);
            self.physics.select_nodes(self.mouse_pos);
            self.mouse_pressed = true;
        }
    }

    fn mouse_button_up_event(&mut self, _: &mut Context, button: MouseButton, _: f32, _: f32) {
        if button == MouseButton::Left {
            self.mouse_pressed = false;
        }
    }

    fn key_down_event(&mut self, _: &mut Context, keycode: KeyCode, _: KeyMods, _: bool) {
        match keycode {
            KeyCode::N => self.can_update = UpdateCommand::OneFrame,
            KeyCode::Space => {
                self.can_update = match self.can_update {
                    UpdateCommand::Continue => UpdateCommand::Stop,
                    _ => UpdateCommand::Continue,
                }
            }
            KeyCode::Escape => self.can_update = UpdateCommand::Quit,
            _ => (),
        }
    }

    fn draw(&mut self, ctx: &mut Context) {
        self.bindings.vertex_buffers[0].update(ctx, &self.physics.get_points());

        let proj = Mat4::orthographic_lh(0.0, WIDTH as f32, HEIGHT as f32, 0.0, 0.0, 1.0);

        ctx.begin_default_pass(Default::default());

        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        ctx.apply_uniforms(&shader::Uniforms { mvp: proj });
        ctx.draw(0, self.physics.num_links() * 2, 1);
        ctx.end_render_pass();

        ctx.commit_frame();
    }
}

fn main() {
    miniquad::start(
        conf::Conf {
            window_width: WIDTH as i32,
            window_height: HEIGHT as i32,
            high_dpi: true,
            ..Default::default()
        },
        |ctx| Box::new(Stage::new(ctx)),
    );
}
