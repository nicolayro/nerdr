//! Tester av `App` uten tmux: `Falsk` tar imot kommandoene og gir registeret
//! testen har bestemt.

mod agenter;
mod klikk;
mod lukke;
mod plass;
mod popup;
mod rydd;
mod svar;
mod tilkobling;

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use nerdr_kjerne::kontroll::Melding;
use nerdr_kjerne::paner::Pane;
use nerdr_kjerne::register::{Claude, Tilstand};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Size;

use crate::omverden::Omverden;
use crate::{App, Hendelse, Svar, taster, tegn};

#[derive(Default)]
struct Logg {
    sendt: Vec<String>,
    register: HashMap<String, Claude>,
    /// Nummeret `koble_igjen` gir, eller `None` når tilkoblingen feiler.
    ny_tilkobling: Option<u64>,
}

struct Falsk(Rc<RefCell<Logg>>);

impl Omverden for Falsk {
    fn send(&mut self, kommando: &str) -> io::Result<()> {
        self.0.borrow_mut().sendt.push(kommando.to_string());
        Ok(())
    }

    fn koble_igjen(&mut self) -> io::Result<u64> {
        self.0.borrow().ny_tilkobling.ok_or_else(|| io::Error::other("ingen tmux"))
    }

    fn for_seg(&mut self, _: Option<&str>, _: &[String]) {}

    fn kopier(&mut self, _: &str) -> bool {
        true
    }

    fn register(&self) -> HashMap<String, Claude> {
        self.0.borrow().register.clone()
    }

    fn utenfor_tmux(&self) -> usize {
        0
    }
}

pub struct Prøve {
    pub app: App,
    logg: Rc<RefCell<Logg>>,
    /// Det tmux har. `svar` lager `list-panes` av dem.
    pub paner: Vec<Pane>,
    /// Svaret på `display-message` fra `hent`.
    pub markør: String,
}

fn linje(p: &Pane) -> String {
    let flagg = |b: bool| if b { "1" } else { "0" };
    [
        &p.sesjon_id,
        &p.sesjon,
        &p.vindu_id,
        &p.vindu,
        &p.id,
        &p.tittel,
        &p.vindu_bredde.to_string(),
        &p.vindu_høyde.to_string(),
        &p.bredde.to_string(),
        &p.høyde.to_string(),
        &p.venstre.to_string(),
        &p.topp.to_string(),
        flagg(p.aktiv),
        &p.oppsett,
        flagg(p.vindu_aktiv),
        &p.vindunavn,
        &p.sett.map(|s| s.to_string()).unwrap_or_default(),
        &p.kommando,
        flagg(p.fra_nerdr),
        flagg(p.zoomet),
        &p.mappe.display().to_string(),
    ]
    .join("\t")
}

impl Prøve {
    pub fn ny() -> Prøve {
        let logg = Rc::new(RefCell::new(Logg::default()));
        let (tx, _) = std::sync::mpsc::channel();
        let mut app = App::ny(Box::new(Falsk(logg.clone())), 1, tx);
        app.prefiks = taster::Prefiks::default();
        app.konfig = PathBuf::from("/finnes/ikke");
        let mut p = Prøve { app, logg, paner: Vec::new(), markør: "0\t0\t1\t0\t0\t0\t0".into() };
        p.endret();
        p
    }

    /// Ett vindu med én pane som står i sin egen sesjon.
    pub fn pane(id: &str, vindu: &str, sesjon: &str) -> Pane {
        let tall = id.trim_start_matches('%');
        Pane {
            sesjon_id: sesjon.into(),
            sesjon: format!("s{tall}"),
            vindu_id: vindu.into(),
            vindu: "0".into(),
            id: id.into(),
            tittel: String::new(),
            vindu_bredde: 80,
            vindu_høyde: 24,
            bredde: 80,
            høyde: 24,
            venstre: 0,
            topp: 0,
            aktiv: true,
            oppsett: String::new(),
            vindu_aktiv: true,
            vindunavn: "claude".into(),
            sett: None,
            kommando: "claude".into(),
            fra_nerdr: false,
            zoomet: false,
            mappe: PathBuf::from("/finnes/ikke"),
        }
    }

    pub fn claude(&mut self, pane: &str, tilstand: Tilstand) {
        let c = Claude { tilstand: Some(tilstand), endret: 1, mappe: None, samtale: None };
        self.logg.borrow_mut().register.insert(pane.into(), c);
    }

    pub fn uten_claude(&mut self, pane: &str) {
        self.logg.borrow_mut().register.remove(pane);
    }

    pub fn svar(&mut self) {
        while let Some(neste) = self.app.kø.front() {
            let linjer = match neste {
                Svar::Paner => self.paner.iter().map(linje).collect(),
                Svar::Markør => vec![self.markør.clone()],
                _ => Vec::new(),
            };
            self.melding(Melding::Svar { ok: true, linjer });
        }
    }

    pub fn endret(&mut self) {
        self.melding(Melding::Endret);
        self.svar();
    }

    pub fn melding(&mut self, m: Melding) {
        let nr = self.app.tilkobling;
        self.app.håndter(Hendelse::Tmux(nr, m));
    }

    pub fn output(&mut self, pane: &str, data: &str) {
        self.melding(Melding::Output { pane: pane.into(), data: data.into() });
    }

    /// `"C-b"`, `"Enter"` eller ett tegn.
    pub fn tast(&mut self, tast: &str) {
        let (mods, navn) = match tast.strip_prefix("C-") {
            Some(rest) => (KeyModifiers::CONTROL, rest),
            None => (KeyModifiers::NONE, tast),
        };
        let kode = match navn {
            "Enter" => KeyCode::Enter,
            "Esc" => KeyCode::Esc,
            n => KeyCode::Char(n.chars().next().expect("en tast")),
        };
        self.app.håndter(Hendelse::Term(Event::Key(KeyEvent::new(kode, mods))));
    }

    pub fn mus(&mut self, kind: MouseEventKind, x: u16, y: u16) {
        let m = MouseEvent { kind, column: x, row: y, modifiers: KeyModifiers::NONE };
        self.app.håndter(Hendelse::Term(Event::Mouse(m)));
    }

    /// Ned og opp, som terminalen sender det. Tegner først, så oppsettet er
    /// det en bruker ser.
    pub fn klikk(&mut self, x: u16, y: u16) {
        self.skjerm();
        self.mus(MouseEventKind::Down(MouseButton::Left), x, y);
        self.mus(MouseEventKind::Up(MouseButton::Left), x, y);
    }

    /// Ruta midt i `tekst` der den står første gang på skjermen.
    pub fn finn(&mut self, tekst: &str) -> (u16, u16) {
        let skjerm = self.skjerm();
        let (y, linje) = skjerm.iter().enumerate().find(|(_, l)| l.contains(tekst)).expect("teksten står på skjermen");
        let start = linje[..linje.find(tekst).unwrap()].chars().count();
        ((start + tekst.chars().count() / 2) as u16, y as u16)
    }

    pub fn sendt(&mut self) -> Vec<String> {
        std::mem::take(&mut self.logg.borrow_mut().sendt)
    }

    pub fn skjerm(&mut self) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("testbackend");
        self.app.forbered_tegning(Size::new(120, 40));
        terminal.draw(|f| tegn::alt(f, &self.app)).expect("tegning");
        let buf = terminal.backend().buffer();
        (0..buf.area.height)
            .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect::<String>().trim_end().to_string())
            .collect()
    }
}

pub fn for_lenge_siden(tid: Duration) -> Instant {
    Instant::now().checked_sub(tid).expect("maskinen har gått lenge nok")
}

#[test]
fn linje_er_det_motsatte_av_fra_linje() {
    let p = Pane { sett: Some(17), fra_nerdr: true, tittel: "✳ Fiks".into(), ..Prøve::pane("%3", "@2", "$1") };
    assert_eq!(Pane::fra_linje(&linje(&p)), Some(p));
}
