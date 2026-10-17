//! Forgrunns- og bakgrunnsfargen til terminalen, så markeringen kan blandes av
//! dem og passe til alle fargetemaer.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode};

pub type Rgb = (u8, u8, u8);

/// DA1 til slutt svarer alle terminaler på, så vi vet at de ikke kan OSC 10/11
/// uten å vente. Tidsgrensen er for den som ikke svarer på noe.
const SPØRSMÅL: &[u8] = b"\x1b]10;?\x1b\\\x1b]11;?\x1b\\\x1b[c";
const VENT: Duration = Duration::from_millis(300);

/// Forgrunn og bakgrunn, eller `None` når terminalen ikke svarer. Må kalles før
/// noe annet leser fra terminalen, ellers kan svaret havne der.
pub fn les() -> Option<(Rgb, Rgb)> {
    let mut tty = OpenOptions::new().read(true).write(true).open("/dev/tty").ok()?;
    enable_raw_mode().ok()?;
    let svar = spør(&mut tty);
    let _ = disable_raw_mode();
    let svar = svar?;
    Some((farge(&svar, "10")?, farge(&svar, "11")?))
}

fn spør(tty: &mut File) -> Option<String> {
    tty.write_all(SPØRSMÅL).ok()?;
    let slutt = Instant::now() + VENT;
    let mut svar = Vec::new();
    let mut buf = [0u8; 256];
    while !har_da1(&svar) {
        let igjen = slutt.saturating_duration_since(Instant::now()).as_millis() as i32;
        let mut fd = libc::pollfd { fd: tty.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        if igjen == 0 || unsafe { libc::poll(&mut fd, 1, igjen) } <= 0 {
            break;
        }
        let n = tty.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        svar.extend_from_slice(&buf[..n]);
    }
    Some(String::from_utf8_lossy(&svar).into_owned())
}

/// Svaret på DA1 er `ESC [ ? … c`.
fn har_da1(svar: &[u8]) -> bool {
    svar.windows(3).position(|w| w == b"\x1b[?").is_some_and(|i| svar[i..].contains(&b'c'))
}

/// Fargen i `ESC ] <nr> ; rgb:RRRR/GGGG/BBBB`, med én til fire hex-sifre per del.
fn farge(svar: &str, nr: &str) -> Option<Rgb> {
    let start = svar.find(&format!("\x1b]{nr};rgb:"))? + nr.len() + 7;
    let rest = &svar[start..];
    let slutt = rest.find(['\x1b', '\x07']).unwrap_or(rest.len());
    let mut deler = rest[..slutt].split('/').map(|d| {
        let verdi = u32::from_str_radix(d, 16).ok()?;
        let maks = 16u32.checked_pow(d.len() as u32)? - 1;
        (1..=4).contains(&d.len()).then(|| (verdi * 255 / maks) as u8)
    });
    Some((deler.next()??, deler.next()??, deler.next()??))
}

/// `andel` av `b` blandet inn i `a`.
pub fn bland(a: Rgb, b: Rgb, andel: f32) -> Rgb {
    let del = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * andel).round() as u8;
    (del(a.0, b.0), del(a.1, b.1), del(a.2, b.2))
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn farge_med_fire_og_to_sifre_og_begge_avslutningene() {
        let svar = "\x1b]10;rgb:ffff/8080/0000\x1b\\\x1b]11;rgb:1e/1e/2e\x07\x1b[?62;22c";
        assert_eq!(farge(svar, "10"), Some((255, 128, 0)));
        assert_eq!(farge(svar, "11"), Some((30, 30, 46)));
    }

    #[test]
    fn uten_osc_svar_er_det_ingen_farge() {
        assert_eq!(farge("\x1b[?62;22c", "11"), None);
        assert_eq!(farge("\x1b]11;rgb:zz/00/00\x07", "11"), None);
    }

    #[test]
    fn da1_avslutter_svaret() {
        assert!(har_da1(b"\x1b]11;rgb:0/0/0\x07\x1b[?1;2c"));
        assert!(!har_da1(b"\x1b]11;rgb:0/0/0\x07\x1b[?1;2"));
    }

    #[test]
    fn blanding_mellom_bakgrunn_og_forgrunn() {
        assert_eq!(bland((0, 0, 0), (200, 100, 50), 0.1), (20, 10, 5));
        assert_eq!(bland((255, 255, 255), (0, 0, 0), 0.5), (128, 128, 128));
    }
}
