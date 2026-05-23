use crate::config::CONFIRMATIONS;

pub struct NoteSmoother {
    stable_note: Option<u8>,
    candidate_note: Option<u8>,
    candidate_count: u8,
}

impl NoteSmoother {
    pub const fn new() -> Self {
        Self {
            stable_note: None,
            candidate_note: None,
            candidate_count: 0,
        }
    }

    pub fn reset(&mut self) {
        self.stable_note = None;
        self.candidate_note = None;
        self.candidate_count = 0;
    }

    pub fn update(&mut self, note: Option<u8>) -> Option<u8> {
        let Some(note) = note else {
            self.reset();
            return None;
        };

        if self.stable_note == Some(note) {
            self.candidate_note = None;
            self.candidate_count = 0;
            return self.stable_note;
        }

        if self.stable_note.is_none() {
            self.stable_note = Some(note);
            return self.stable_note;
        }

        if self.candidate_note == Some(note) {
            self.candidate_count = self.candidate_count.saturating_add(1);
        } else {
            self.candidate_note = Some(note);
            self.candidate_count = 1;
        }

        if self.candidate_count >= CONFIRMATIONS {
            self.stable_note = Some(note);
            self.candidate_note = None;
            self.candidate_count = 0;
        }

        self.stable_note
    }
}
