use pixels::{Error, Pixels, SurfaceTexture};
use rand::Rng;
use rayon::prelude::*;
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use std::time::Instant;
use winit_input_helper::WinitInputHelper;

#[derive(Copy, Clone)]
struct DifCell {
    a: f32,
    b: f32,
}

// // Metosis
// const DA: f32 = 1.0;
// const DB: f32 = 0.5;
// const K: f32 = 0.0649;
// const F: f32 = 0.0367;
// const T: f32 = 1.0;

// Corals
const DA: f32 = 1.0;
const DB: f32 = 0.5;
const K: f32 = 0.062;
const F: f32 = 0.0545;
const T: f32 = 0.3;

const CORNER: f32 = 0.05;
const SIDE: f32 = 0.2;
const LAP: [[f32; 3]; 3] = [
    [CORNER, SIDE, CORNER],
    [SIDE, -1.0, SIDE],
    [CORNER, SIDE, CORNER],
];

impl DifCell {
    fn from(a: f32, b: f32) -> Self {
        DifCell { a, b }
    }
    fn update(&self, lap_a: f32, lap_b: f32, f:f32, k:f32) -> Self {
        let abb = self.a * self.b * self.b;
        let a = self.a + (DA * lap_a - abb + f * (1.0 - self.a)) * T;
        let b = self.b + (DB * lap_b + abb - (k + f) * self.b) * T;

        DifCell::from(a, b)
    }
    fn color(&self) -> [u8; 4] {
        let red = (self.a * 255.0) as u8;
        let blue = (self.b * 255.0) as u8;
        [red, 0, blue, 255]
    }
}

struct CellGrid {
    switch: bool,
    field1: Vec<DifCell>,
    field2: Vec<DifCell>,
    neighbours: Vec<[usize;9]>,
    fk_v: Vec<(f32,f32)>,
}

impl CellGrid {
    fn fk(width: usize, height: usize, pos: usize) -> (f32,f32) {
        let (i,j) = (pos / width, pos % width);
        let (i_f, j_f) = (i as f32 / height as f32, j as f32 / width as f32);
        ( i_f * 0.3, j_f * 0.08)
        // (F,K)
    }
    fn new(width: usize, height: usize) -> Self {
        let mut rng = rand::thread_rng();
        let mut field1 = vec![DifCell::from(1.0, 0.0); width * height];
        for i in 0..height {
            for j in 0..width {
                // if (10 < i && i < 20) && (10 < j && j < 20)  {
                // if (height / 20 * 9 < i && i < height / 20 * 11) && (width / 20 * 9 < j && j < width / 20 * 11) && rng.gen::<f32>() > 0.2 {
                // if rng.gen::<f32>() > 0.9 {
                if (i % 20 < 10) && (j % 20 < 10) && rng.gen::<f32>() > 0.9 {
                    field1[i * width + j] = DifCell::from(0.0, 1.0);
                }
            }
        }
        let field2 = vec![DifCell::from(1.0, 0.0); width * height];
        let mut neighbours = vec![[0;9]; width * height];
        
        for i in 0..height {
            for j in 0..width {
                for ps in 0..9 {
                    let (ii,jj) = (ps / 3,ps % 3);
                    let iii =(i + ii + height - 1) % height;
                    let jjj =(j + jj + width - 1) % width;
                    let pos = iii * width + jjj;
                    neighbours[i * width + j][ps] = pos;
                }
            }
        }

        let mut fk_v = vec![(0.0,0.0); width * height];

        for pos in 0..(width * height) {
            fk_v[pos] = Self::fk(width,height ,pos );
        }
        
        let switch = false;
        Self {
            switch,
            field1,
            field2,
            neighbours,
            fk_v,
        }
    }
    fn get_field(&self) -> &Vec<DifCell> {
        if self.switch {
            &self.field2
        } else {
            &self.field1
        }
    }
    fn get_fields(&mut self) -> (&mut Vec<DifCell>,&Vec<DifCell>, &Vec<[usize;9]>, &Vec<(f32,f32)>) {
        if self.switch {
            (&mut self.field2, &self.field1, &self.neighbours, &self.fk_v)
        } else {
            (&mut self.field1, &self.field2, &self.neighbours, &self.fk_v)
        }
    }
    fn get_lap(pos: usize, buff: &Vec<DifCell>, neighbours: &Vec<[usize;9]>) -> (f32, f32) {
        let mut lap_a = 0.0;
        let mut lap_b = 0.0;
        for ii in 0..3 {
            for jj in 0..3 {
                lap_a += LAP[ii][jj] * buff[neighbours[pos][ii * 3 + jj]].a;
                lap_b += LAP[ii][jj] * buff[neighbours[pos][ii * 3 + jj]].b;
            }
        }
        (lap_a, lap_b)
    }
    fn update(&mut self) {
        self.switch = !self.switch;
        let (field, buff,neighbours, fk_v) = self.get_fields();

        *field = buff
            .into_par_iter().enumerate()
            .map(|(pos, elem)| {
                let (lap_a, lap_b) = Self::get_lap(pos,buff, neighbours);
                let (f,k) = fk_v[pos];
                elem.update(lap_a, lap_b,f,k)
            })
            .collect();
    }
    fn draw(&self, screen: &mut [u8]) {
        for (cell, pix) in self.get_field().iter().zip(screen.chunks_exact_mut(4)) {
            let color = cell.color();
            pix.copy_from_slice(&color);
        }
    }
}

const WIDTH: usize = 1200;
const HEIGHT: usize = 900;

const SCALER: f64 = 1.0;

const TARGET_FPS: u64 = 60;
const TPF: u64 = 10;

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * SCALER, HEIGHT as f64 * SCALER);
        WindowBuilder::new()
            .with_title("reaction duffusion")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture)?
    };

    let mut cells  = CellGrid::new(WIDTH, HEIGHT);

    event_loop.run(move |event, _, control_flow| {
        let start_time = Instant::now();
        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if input.key_pressed(VirtualKeyCode::Space) {
                cells.update();
            }

            let start_compute = Instant::now();
            for _ in 0..TPF {
                cells.update();
            }
            let compute_time = Instant::now().duration_since(start_compute).as_secs_f32();

            let start_draw = Instant::now();

            window.request_redraw();
            let draw_time = Instant::now().duration_since(start_draw).as_secs_f32();


            let elapsed_time_f32 = Instant::now().duration_since(start_time).as_secs_f32();


            let fps = 1.0 / elapsed_time_f32;

            println!("tpf: {} , fps: {:.1} , loop time: {:.2} ms , total compute time: {:.2} ms , compute time per tick: {:.2} ms , draw time: {:.2} ms", 
                TPF, 
                fps, 
                elapsed_time_f32 * 1000.0,
                compute_time * 1000.0,
                compute_time * 1000.0 / TPF as f32,
                draw_time * 1000.0
            );

            let elapsed_time = (elapsed_time_f32 * 1000.0) as u64;
                let wait_millis = match 1000 / TARGET_FPS >= elapsed_time {
                true => 1000 / TARGET_FPS - elapsed_time,
                false => 0,
            };
            let new_inst = start_time + std::time::Duration::from_millis(wait_millis);

            *control_flow = ControlFlow::WaitUntil(new_inst);
        }

        if let Event::RedrawRequested(_) = event {
            cells.draw(pixels.frame_mut());
            if let Err(_err) = pixels.render() {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }
    });
}
