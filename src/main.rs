// simplest game you can write with graphics, sound and user interaction
extern crate piston_window;
extern crate music;
extern crate find_folder;
extern crate rand;
extern crate time;

use piston_window::*;
use rand::Rng;

const COURT_HEIGHT: f64 = 700.0;
const COURT_WIDTH: f64 = 900.0;
const BAT_WIDTH: f64 = 130.0;
const BAT_THICKNESS: f64 = 19.0;
const BAT_Y: f64 = COURT_HEIGHT - 50.0;
const Y_VEL: f64 = 300.0;
const X_VEL_SHALLOW: f64 = Y_VEL * 1.5;
const X_VEL_NORMAL: f64 = Y_VEL;
const X_VEL_STEEP: f64 = Y_VEL * 0.5;
const MAX_BALLS: usize = 5;
const NEW_BALL_INTERVAL: u32 = 5;
const BALL_RADIUS: f64 = 10.0;
const BALL_RADIUS_SQ: f64 = BALL_RADIUS * BALL_RADIUS;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Sound {
    HitTable,
    HitBat,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Music {}

enum GameMode {
    StartUp,
    Playing,
    GameOver,
}

struct Score {
    points: u32,
    need_new_ball: bool,
}

impl Score {
    fn new() -> Score {
        Score {points: 0, need_new_ball: false}
    }

    fn reset(&mut self) {
        self.points = 0;
        self.need_new_ball = false;
    }

    fn increment(&mut self) {
        self.points += 1;
        self.need_new_ball = self.points % NEW_BALL_INTERVAL == 0;
    }

    fn new_ball_tripped(&mut self) -> bool {
        let result = self.need_new_ball;
        self.need_new_ball = false;
        result
    }

    fn render(&self, c: Context, g: &mut G2d, glyphs: &mut Glyphs) {
        text::Text::new_color([0.3, 0.3, 0.3, 1.0], 128).draw(
            &self.points.to_string(),
            glyphs,
            &c.draw_state,
            c.transform.trans((COURT_WIDTH / 2.0) - 50.0, COURT_HEIGHT / 2.0),
            g
            ).unwrap();
    }
}

struct AxisMotion {
    pos: f64,
    old_pos: f64,
    vel: f64,
}

impl AxisMotion {
    fn new() -> AxisMotion {
        AxisMotion{pos: 0.0, old_pos: 0.0, vel: 0.0}
    }
}

struct Edge {
    pos: f64,
    start: f64,
    end: f64,
}

impl Edge {
    fn infinite(pos: f64) -> Edge {
        // approximations of -ve and +ve infinity ;-)
        Edge{pos, start: -10000.0, end: 10000.0}
    }

    fn from_width(pos:f64, start: f64, width: f64) -> Edge {
        Edge{pos, start, end: start + width}
    }
}

struct Ball {
    x: AxisMotion,
    y: AxisMotion,
    colour: [f32; 4],
    in_play: bool,
}

impl Ball {
    fn new(colour: [f32; 4]) -> Ball {
        Ball{x: AxisMotion::new(), y: AxisMotion::new(), colour, in_play: false}
    }

    fn strike(am: &AxisMotion, orth_am: &AxisMotion, edge: &Edge) -> Option<f64> {
        let adj_strike_edge = if am.vel > 0.0 {edge.pos - BALL_RADIUS}
                              else {edge.pos + BALL_RADIUS};
        if (am.vel > 0.0 && am.pos >= adj_strike_edge) ||
           (am.vel < 0.0 && am.pos <= adj_strike_edge) {
            let strike_pos = (adj_strike_edge - am.old_pos) * (orth_am.pos - orth_am.old_pos) /
                             (am.pos - am.old_pos) + orth_am.old_pos;
            if strike_pos >= edge.start && strike_pos <= edge.end {
                return Some(strike_pos);
            }
        }
        return None;
    }

    fn normal_rebound(am: &mut AxisMotion, orth_am: &AxisMotion, edge: &Edge) -> bool {
        match Ball::strike(am, orth_am, &edge) {
            Some(_) => {
                am.vel = - am.vel;
                am.pos = 2.0 * edge.pos - am.pos + (am.vel.signum() * 2.0 * BALL_RADIUS);
                true
            },
            _ => {
                false
            },
        }
    }

    fn reposition_overshoot(&mut self, strike_x: f64, strike_y: f64, overshoot: f64) {
        self.x.pos = strike_x + self.x.vel * overshoot;
        self.y.pos = strike_y + self.y.vel * overshoot;
    }

    fn bat_face_rebound(&mut self, bat_x: f64, speedup: f64) -> bool {
        match Ball::strike(&self.y, &self.x, &Edge::from_width(BAT_Y, bat_x, BAT_WIDTH)) {
            Some(strike_x) => {
                // these are not the traditional pong bat rebound rules...
                let face_pos = if self.x.vel > 0.0 {strike_x - bat_x}
                               else {bat_x + BAT_WIDTH - strike_x};
                let overshoot = (self.y.pos - BAT_Y + BALL_RADIUS) / self.y.vel;
                if face_pos < 0.3 * BAT_WIDTH {
                    self.x.vel = self.x.vel.signum() * X_VEL_STEEP * speedup;
                }
                else if face_pos > 0.7 * BAT_WIDTH {
                    self.x.vel = self.x.vel.signum() * X_VEL_SHALLOW * speedup;
                }
                else {
                    self.x.vel = self.x.vel.signum() * X_VEL_NORMAL * speedup;
                }
                self.y.vel = - Y_VEL * speedup;
                self.reposition_overshoot(strike_x, BAT_Y - BALL_RADIUS, overshoot);
                true
            },
            _ => {
                false
            }
        }
    }

    fn corner_strike(&self, cx: f64) -> Option<(f64, f64)> {
        if self.y.pos > BAT_Y - BALL_RADIUS && self.y.old_pos < BAT_Y {
            let dx = self.x.pos - self.x.old_pos;
            let dy = self.y.pos - self.y.old_pos;
            let dr_sq = square(dx) + square(dy);
            let d = (self.x.old_pos - cx) * (self.y.pos - BAT_Y) - 
                (self.x.pos - cx) * (self.y.old_pos - BAT_Y);
            let disc = BALL_RADIUS_SQ * dr_sq - square(d);
            if disc > 0.0 {
                let strike_y = BAT_Y + (-d * dx - dy.abs() * disc.sqrt()) / dr_sq;
                let strike_x = self.x.pos - (self.y.pos - strike_y) * (self.x.pos - self.x.old_pos) /
                    (self.y.pos - self.y.old_pos);
                if self.y.pos >= strike_y && self.y.old_pos < strike_y {
                    return Some((strike_x, strike_y));
                }
            }
        }
        return None;
    }

    fn bat_corner_rebound(&mut self, bat_x: f64, speedup: f64) -> bool {
        // left bat corner
        if let Some((cx, cy)) = self.corner_strike(bat_x) {
            let overshoot = (self.y.pos - cy) / self.y.vel;
            self.y.vel = -Y_VEL * speedup;
            self.x.vel = -X_VEL_SHALLOW * speedup;
            self.reposition_overshoot(cx, cy, overshoot);
            return true;
        }
        // right bat corner
        else if let Some((cx, cy)) = self.corner_strike(bat_x + BAT_WIDTH) {
            let overshoot = (self.y.pos - cy) / self.y.vel;
            self.y.vel = -Y_VEL * speedup;
            self.x.vel = X_VEL_SHALLOW * speedup;
            self.reposition_overshoot(cx, cy, overshoot);
            return true;
        }
        return false;
    }

    fn bat_sound() {
        music::play_sound(&Sound::HitBat, music::Repeat::Times(0), music::MAX_VOLUME);
    }

    fn table_sound() {
        music::play_sound(&Sound::HitTable, music::Repeat::Times(0), music::MAX_VOLUME);
    }

    fn update(&mut self, dt: f64, bat_x: f64, score: &mut Score, speedup: f64) {
        if self.in_play {
            self.x.old_pos = self.x.pos;
            self.y.old_pos = self.y.pos;
            self.x.pos += &self.x.vel * dt;
            self.y.pos += &self.y.vel * dt;

            if (self.x.vel > 0.0 &&
              Ball::normal_rebound(&mut self.x, &self.y, &Edge::infinite(COURT_WIDTH))) ||
              (self.x.vel < 0.0 && 
              Ball::normal_rebound(&mut self.x, &self.y, &Edge::infinite(0.0))) {
                Ball::table_sound();
            }
            if self.y.vel < 0.0 &&
              Ball::normal_rebound(&mut self.y, &self.x, &Edge::infinite(0.0)) {
                Ball::table_sound();
            }
            else if self.y.vel > 0.0 && self.y.old_pos < BAT_Y - BALL_RADIUS &&
                self.bat_face_rebound(bat_x, speedup) {
                Ball::bat_sound();
                score.increment();
            }
            if (self.y.pos >= BAT_Y && self.x.vel > 0.0 &&
              Ball::normal_rebound(&mut self.x, &self.y, &Edge::from_width(bat_x, BAT_Y, BAT_THICKNESS))) ||
              (self.y.pos >= BAT_Y && self.x.vel < 0.0 &&
              Ball::normal_rebound(&mut self.x, &self.y, &Edge::from_width(bat_x + BAT_WIDTH, BAT_Y, BAT_THICKNESS))) {
                Ball::bat_sound();
            }
            else if self.bat_corner_rebound(bat_x, speedup) {
                Ball::bat_sound();
                score.increment();
            }
            // At present, we pretend that the bat dematerialises from one position
            // and rematerialises somewhere else in the next frame.
            // What might be more satsfying is to consider the journey between and
            // allow a save via a corner hit or a sideways "push" where the ball has
            // gone too far.
            if self.y.pos > COURT_HEIGHT + BALL_RADIUS {
                self.in_play = false;
            }
        }
    }

    fn serve(&mut self, speedup: f64) {
        self.x.vel = (if rand::thread_rng().gen() {X_VEL_NORMAL} else {- X_VEL_NORMAL}) * speedup;
        self.y.vel = Y_VEL * speedup;
        self.x.pos = rand::thread_rng().gen_range(BALL_RADIUS as u32 + 1, (COURT_WIDTH - BALL_RADIUS) as u32) as f64;
        self.y.pos = rand::thread_rng().gen_range(BALL_RADIUS as u32 + 1, COURT_HEIGHT as u32 / 3) as f64;
        self.x.old_pos = self.x.pos - self.x.vel;
        self.y.old_pos = self.y.pos - self.y.vel;
        self.in_play = true;
    }

    fn render(&self, c: Context, g: &mut G2d) {
        if self.in_play {
            let ball_shape = ellipse::circle(0.0, 0.0, BALL_RADIUS);
            ellipse(self.colour, ball_shape, c.transform.trans(self.x.pos, self.y.pos), g);
        }
    }
}

struct Balls {
    ball: [Ball; MAX_BALLS]
}

impl Balls {
    fn new() -> Balls {
         Balls {ball: [Ball::new([1.0, 1.0, 1.0, 1.0]),
                       Ball::new([0.9, 0.65, 0.89, 1.0]),
                       Ball::new([0.9, 0.71, 0.5, 1.0]),
                       Ball::new([0.75, 0.91, 0.63, 1.0]),
                       Ball::new([0.73, 0.89, 0.9, 1.0])
         ]}
    }

    fn update(&mut self, dt: f64, bat_x: f64, score: &mut Score, speedup: f64) {
        for i in 0..MAX_BALLS {
            self.ball[i].update(dt, bat_x, score, speedup);
        }
        if score.new_ball_tripped() {
            self.serve_new_ball(speedup);
        }
    }

    fn render(&self, c: Context, g: &mut G2d) {
        for i in 0..MAX_BALLS {
            self.ball[i].render(c, g);
        }
    }

    fn serve_new_ball(&mut self, speedup: f64) {
        for i in 0..MAX_BALLS {
            if ! self.ball[i].in_play {
                self.ball[i].serve(speedup);
                break;
            }
        }
    }

    fn in_play(&self) -> bool {
        for i in 0..MAX_BALLS {
            if self.ball[i].in_play {
                return true;
            }
        }
        return false;
    }
}

fn square(n: f64) -> f64 {
    n * n
}

fn render_text(c: Context, g: &mut G2d, glyphs: &mut Glyphs, x: f64, y: f64, txt: &str) {
    text::Text::new_color([1.0, 1.0, 1.0, 1.0], 32).draw(
        txt,
        glyphs,
        &c.draw_state,
        c.transform.trans(x, y),
        g
        ).unwrap();
}

fn main() {
    let opengl = OpenGL::V2_1;
    let mut window: PistonWindow = WindowSettings::new(
        "Pongish",
        (COURT_WIDTH as u32, COURT_HEIGHT as u32)
    )
    .exit_on_esc(true)
    .opengl(opengl)
    .build()
    .unwrap();

    window.set_capture_cursor(true);

    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets").unwrap();

    // assets - images
    let bat_image = assets.join("bat.png");
    let bat_image = Texture::from_path(
            &mut window.factory,
            &bat_image,
            Flip::None,
            &TextureSettings::new()
        ).unwrap();

    // assets - sounds
    let hit_table = assets.join("hit-table.wav");
    let hit_bat = assets.join("hit-bat.wav");

    // assets - font
    let ref font = assets.join("FiraSans-Regular.ttf");
    let factory = window.factory.clone();
    let mut glyphs = Glyphs::new(font, factory, TextureSettings::new()).unwrap();

    let mut bat_x = (COURT_WIDTH + BAT_WIDTH) / 2.0;
    let mut score = Score::new();
    let mut game_mode = GameMode::StartUp;

    let mut balls = Balls::new();

    let mut accumt: f64 = 0.0;
    let mut start_time = time::Timespec {sec: 0, nsec: 0};
    let mut end_time  = time::Timespec {sec: 0, nsec: 0};

    // need to rein in ups as performs poorly on my hardware
    // delta time supplied by update_args does not adjust to actual time elapsed but what
    // it was supposed to be according to ups - if we overrun we get slow down
    window.set_ups(60); 

    #[cfg(debug_assertions)]
    window.set_ups(20); 

    music::start::<Music, Sound, _>(16, || {
        music::bind_sound_file(Sound::HitTable, hit_table);
        music::bind_sound_file(Sound::HitBat, hit_bat);

        while let Some(e) = window.next() {
            match game_mode {
                GameMode::StartUp => {
                    window.draw_2d(&e, |c, g| {
                        clear([0.0, 0.0, 0.0, 1.0], g);

                        if let Some(_) = e.render_args() {
                            // pre-render all digits as they take longer the first time
                            for digit in 0..10 {
                                text::Text::new_color([0.3, 0.3, 0.3, 1.0], 128).draw(
                                    &digit.to_string(),
                                    &mut glyphs,
                                    &c.draw_state,
                                    c.transform.trans(-200.0, -200.0),
                                    g
                                    ).unwrap();
                            }

                            render_text(c, g, &mut glyphs, 200.0, 400.0, "Pongish - somewhat Pong-like");
                            render_text(c, g, &mut glyphs, 200.0, 475.0, "Press space to play");
                            render_text(c, g, &mut glyphs, 200.0, 550.0, "Use the mouse to control the bat");
                            render_text(c, g, &mut glyphs, 200.0, 625.0, "Press escape to exit at any time");
                        }
                    });
                    if let Some(Button::Keyboard(Key::Space)) = e.release_args() {
                        game_mode = GameMode::Playing;
                        score.reset();
                        balls.ball[0].serve(1.0);
                        accumt = 0.0;
                        start_time = time::get_time();
                    }
                },
                GameMode::Playing => {
                    if let Some(_) = e.render_args() {
                        window.draw_2d(&e, |c, g| {
                            clear([0.0, 0.0, 0.0, 1.0], g);
                            score.render(c, g, &mut glyphs);
                            balls.render(c, g);
                            image(&bat_image, c.transform.trans(bat_x, BAT_Y), g);
                        });
                    }

                    e.mouse_relative(|x, _| {
                        bat_x += x;
                        if bat_x < - BAT_WIDTH {
                            bat_x = - BAT_WIDTH;
                        }
                        if bat_x > COURT_WIDTH {
                            bat_x = COURT_WIDTH;
                        }
                    });

                    if let Some(u) = e.update_args() {
                        balls.update(u.dt, bat_x, &mut score, 1.0 + (accumt / 200.0));
                        accumt += u.dt;

                        if ! balls.in_play() {
                            game_mode = GameMode::GameOver;
                            end_time = time::get_time();
                            let elapsed_time: f64 = (end_time.sec - start_time.sec) as f64 +
                                (end_time.nsec as f64 / 1000000000.0) -
                                (start_time.nsec as f64 / 1000000000.0);
                            println!("Lost time: {} of {}", elapsed_time - accumt, elapsed_time);
                        }
                    }

                },
                GameMode::GameOver => {
                    if let Some(_) = e.render_args() {
                        window.draw_2d(&e, |c, g| {
                            clear([0.0, 0.0, 0.0, 1.0], g);
                            score.render(c, g, &mut glyphs);
                            render_text(c, g, &mut glyphs, 200.0, 400.0, "Game Over");
                            render_text(c, g, &mut glyphs, 200.0, 475.0, "Press space to play again");
                            render_text(c, g, &mut glyphs, 200.0, 550.0, "Press escape to exit at any time");
                        });
                    }
                    if let Some(Button::Keyboard(Key::Space)) = e.release_args() {
                        game_mode = GameMode::Playing;
                        score.points = 0;
                        balls.ball[0].serve(1.0);
                        accumt = 0.0;
                        start_time = time::get_time();
                    }
                },
            }
        }
    });
}
