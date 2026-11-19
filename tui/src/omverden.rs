//! Det `App` gjør utenfor seg selv. I testene er det en falsk som husker hva
//! som ble sendt, så en `App` kan lages uten tmux, utklippstavle og register.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::Child;

use nerdr_kjerne::kontroll::{self, Kontroll};
use nerdr_kjerne::register::{self, Claude};

use crate::markering;

pub trait Omverden {
    /// Svaret kommer som en `Melding::Svar`, i samme rekkefølge som kommandoene.
    fn send(&mut self, kommando: &str) -> io::Result<()>;
    /// Nummeret meldingene fra den nye tilkoblingen får.
    fn koble_igjen(&mut self) -> io::Result<u64>;
    /// `pane` er mål for kommandoene, når nerdr har en.
    fn for_seg(&mut self, pane: Option<&str>, linjer: &[String]);
    fn kopier(&mut self, tekst: &str) -> bool;
    fn register(&self) -> HashMap<String, Claude>;
    fn utenfor_tmux(&self) -> usize;
}

pub struct Tmux {
    kontroll: Kontroll,
    /// tmux kobler fra klienten når sesjonen den står i, forsvinner, som når
    /// det siste vinduet i den lukkes. Da kobler nerdr til igjen.
    koble: Box<dyn FnMut() -> io::Result<(Kontroll, u64)>>,
    socket: Option<String>,
    /// Klientene bindingene fra tmux-oppsettet kjører i, se `kontroll::for_seg`.
    for_seg: Vec<Child>,
    /// Registeret til Claude Code ligger i `sessions` her.
    konfig: PathBuf,
}

impl Tmux {
    pub fn ny(
        kontroll: Kontroll,
        koble: Box<dyn FnMut() -> io::Result<(Kontroll, u64)>>,
        socket: Option<String>,
        konfig: PathBuf,
    ) -> Tmux {
        Tmux { kontroll, koble, socket, for_seg: Vec::new(), konfig }
    }
}

impl Omverden for Tmux {
    fn send(&mut self, kommando: &str) -> io::Result<()> {
        self.kontroll.send(kommando)
    }

    fn koble_igjen(&mut self) -> io::Result<u64> {
        let (kontroll, nr) = (self.koble)()?;
        self.kontroll = kontroll;
        Ok(nr)
    }

    fn for_seg(&mut self, pane: Option<&str>, linjer: &[String]) {
        self.for_seg.retain_mut(|barn| matches!(barn.try_wait(), Ok(None)));
        if let Ok(barn) = kontroll::for_seg(self.socket.as_deref(), pane, linjer) {
            self.for_seg.push(barn);
        }
    }

    fn kopier(&mut self, tekst: &str) -> bool {
        markering::kopier(tekst)
    }

    fn register(&self) -> HashMap<String, Claude> {
        register::les(&self.konfig.join("sessions"))
    }

    fn utenfor_tmux(&self) -> usize {
        register::utenfor_tmux(&self.konfig.join("sessions"))
    }
}

impl Drop for Tmux {
    fn drop(&mut self) {
        for barn in &mut self.for_seg {
            let _ = barn.kill();
            let _ = barn.wait();
        }
    }
}
