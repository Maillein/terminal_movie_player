use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};
use opencv::core::{Size, Vec3b};
#[allow(unused_imports)]
use opencv::imgproc::INTER_NEAREST;
use opencv::imgproc::{INTER_CUBIC, INTER_LANCZOS4};
use opencv::videoio::{CAP_FFMPEG, CAP_PROP_FPS, CAP_PROP_FRAME_HEIGHT, CAP_PROP_FRAME_WIDTH};
use opencv::{
    hub_prelude::VideoCaptureTraitConst, imgproc, prelude::*, videoio::VideoCapture, Result,
};
use std::fs::File;
use std::io::{stdout, BufWriter, Write};
use std::mem;
use std::os::unix::io::IntoRawFd;
use std::thread::sleep;
use std::time::{Duration, Instant};

const SKIP_FRAME_RATIO: u64 = 3;
const MOVIE_NAME: &str = "no_no_no.mkv";

#[allow(dead_code)]
struct VideoInfo {
    width: f64,
    height: f64,
    aspect_ratio: f64,
    fps: f64,
}

impl VideoInfo {
    fn new(video: &mut VideoCapture) -> Self {
        let width = video.get(CAP_PROP_FRAME_WIDTH).unwrap();
        let height = video.get(CAP_PROP_FRAME_HEIGHT).unwrap();
        let fps = video.get(CAP_PROP_FPS).unwrap();

        Self {
            width,
            height,
            aspect_ratio: width / height,
            fps,
        }
    }
}

pub fn terminal_size() -> Option<winsize> {
    // STDOUT_FILENOか/dev/ttyを利用する
    let fd = if let Ok(file) = File::open("/dev/tty") {
        file.into_raw_fd()
    } else {
        STDOUT_FILENO
    };

    // ファイルディスクリプタに対してTIOCGWINSZをシステムコール
    let mut ws: winsize = unsafe { mem::zeroed() };
    if unsafe { ioctl(fd, TIOCGWINSZ, &mut ws) } == -1 {
        None
    } else {
        Some(ws)
    }
}

pub fn true_color(c: &[u8]) -> String {
    format!("\x1b[38;2;{:>03};{:>03};{:>03}m", c[0], c[1], c[2])
}

fn main() -> Result<()> {
    // 動画の読み込み
    let mut video = VideoCapture::from_file(format!("movies/{}", MOVIE_NAME).as_str(), CAP_FFMPEG)?;
    let info = VideoInfo::new(&mut video);
    let time_per_frame =
        Duration::from_nanos(SKIP_FRAME_RATIO * 1_000_000_000 / info.fps.round() as u64);

    // ターミナルのサイズの取得
    let (t_w, t_h) = if let Some(ws) = terminal_size() {
        let mut w = ws.ws_col as f64;
        let mut h = ws.ws_row as f64;
        if w > h * info.aspect_ratio * 2.0 {
            w = h * info.aspect_ratio * 2.0;
        } else {
            h = w * 2.0 / info.aspect_ratio;
        }
        (w as i32, h as i32)
    } else {
        ((20.0 * info.aspect_ratio) as i32, 20)
    };

    // 出力
    let out = stdout();
    let mut out = BufWriter::with_capacity(4096, out.lock());

    // メインループ
    let mut frame = Mat::default();
    let mut frame_counter = 0;
    while video.read(&mut frame)? {
        frame_counter += 1;
        if frame_counter % SKIP_FRAME_RATIO != 0 {
            continue;
        }
        let start_time = Instant::now();
        let mut image = Mat::default();
        imgproc::resize(
            &frame,
            &mut image,
            Size::new(t_w, t_h),
            0.0,
            0.0,
            INTER_LANCZOS4,
        )?;

        out.write(b"\x1b[H").unwrap();
        out.write(b"\x1b[38;2;150;150;150m").unwrap();
        for y in 0..image.rows() {
            for x in 0..image.cols() {
                let px: &Vec3b = image.at_2d(y, x).unwrap();
                out.write_fmt(format_args!(
                    "\x1b[48;2;{};{};{}m ",
                    px.0[2], px.0[1], px.0[0],
                ))
                .unwrap();
            }
            out.write(b"\x1b[0m\n").unwrap();
        }

        out.flush().unwrap();

        // ループ開始時からの経過時間
        let passed_time = start_time.elapsed();
        if passed_time < time_per_frame {
            // 時間調整
            sleep(time_per_frame - passed_time);
        }
    }
    video.release()?;
    out.write(b"\x1b[0m").unwrap();
    out.flush().unwrap();
    Ok(())
}
