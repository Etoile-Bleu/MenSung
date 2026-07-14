//! Application state and the state machine that drives the interactive
//! interface: two drug input fields, a confirmation step for any non-exact
//! match, and a results screen. There is no path from typed text to a
//! result that skips the confirmation step, per the no-silent-correction
//! rule in MEDICAL_DATA_POLICY.md.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mensung_core::{check_interactions, lookup_drug, Candidate, LookupOutcome};
use mensung_db::{Database, DrugRecord, InteractionRecord};
use mensung_domain::DrugId;

const FIELD_COUNT: usize = 2;

#[derive(Debug)]
pub(crate) enum Screen<'a> {
    Input,
    Candidates {
        field: usize,
        candidates: Vec<Candidate<'a>>,
        selected: usize,
    },
    NoMatch {
        field: usize,
    },
    Results {
        interactions: Vec<InteractionRecord<'a>>,
    },
    Error(String),
}

pub(crate) struct App<'a> {
    db: &'a Database<'a>,
    inputs: [String; FIELD_COUNT],
    focused: usize,
    resolved: [Option<DrugRecord<'a>>; FIELD_COUNT],
    screen: Screen<'a>,
    should_quit: bool,
}

impl<'a> App<'a> {
    pub(crate) fn new(db: &'a Database<'a>) -> Self {
        Self {
            db,
            inputs: [String::new(), String::new()],
            focused: 0,
            resolved: [None, None],
            screen: Screen::Input,
            should_quit: false,
        }
    }

    pub(crate) fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub(crate) fn screen(&self) -> &Screen<'a> {
        &self.screen
    }

    pub(crate) fn inputs(&self) -> &[String; FIELD_COUNT] {
        &self.inputs
    }

    pub(crate) fn focused(&self) -> usize {
        self.focused
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        match self.screen {
            Screen::Input => self.handle_input_key(key),
            Screen::Candidates { .. } => self.handle_candidates_key(key),
            Screen::NoMatch { .. } | Screen::Results { .. } | Screen::Error(_) => {
                self.handle_dismiss_key(key)
            }
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab | KeyCode::Down => self.focused = (self.focused + 1) % FIELD_COUNT,
            KeyCode::BackTab | KeyCode::Up => {
                self.focused = (self.focused + FIELD_COUNT - 1) % FIELD_COUNT
            }
            KeyCode::Backspace => {
                self.inputs[self.focused].pop();
            }
            KeyCode::Char(c) => self.inputs[self.focused].push(c),
            KeyCode::Enter => self.submit(),
            _ => {}
        }
    }

    fn handle_candidates_key(&mut self, key: KeyEvent) {
        let Screen::Candidates {
            field,
            candidates,
            selected,
        } = &mut self.screen
        else {
            return;
        };

        match key.code {
            KeyCode::Esc => self.screen = Screen::Input,
            KeyCode::Down => *selected = (*selected + 1).min(candidates.len() - 1),
            KeyCode::Up => *selected = selected.saturating_sub(1),
            KeyCode::Enter => {
                let field = *field;
                let chosen = candidates[*selected].drug();
                self.inputs[field] = chosen.name().to_string();
                self.resolved[field] = Some(chosen);
                self.screen = Screen::Input;
                self.resolve_next();
            }
            _ => {}
        }
    }

    fn handle_dismiss_key(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
            self.inputs = [String::new(), String::new()];
            self.resolved = [None, None];
            self.focused = 0;
            self.screen = Screen::Input;
        }
    }

    fn submit(&mut self) {
        if self.inputs.iter().any(|field| field.trim().is_empty()) {
            return;
        }
        self.resolved = [None, None];
        self.resolve_next();
    }

    fn resolve_next(&mut self) {
        for field in 0..FIELD_COUNT {
            if self.resolved[field].is_some() {
                continue;
            }

            match lookup_drug(self.db, self.inputs[field].trim()) {
                Ok(LookupOutcome::ExactMatch(drug)) => {
                    self.resolved[field] = Some(drug);
                }
                Ok(LookupOutcome::Candidates(candidates)) => {
                    self.screen = Screen::Candidates {
                        field,
                        candidates,
                        selected: 0,
                    };
                    return;
                }
                Ok(LookupOutcome::NoMatch) => {
                    self.screen = Screen::NoMatch { field };
                    return;
                }
                Err(err) => {
                    self.screen = Screen::Error(err.to_string());
                    return;
                }
            }
        }

        self.check_resolved_interactions();
    }

    fn check_resolved_interactions(&mut self) {
        let ids: Vec<DrugId> = self
            .resolved
            .iter()
            .map(|drug| {
                drug.as_ref()
                    .expect("resolve_next only reaches here once every field is resolved")
                    .id()
            })
            .collect();

        match check_interactions(self.db, &ids) {
            Ok(interactions) => self.screen = Screen::Results { interactions },
            Err(err) => self.screen = Screen::Error(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mensung_db::test_support::{build_men_file, TestDrug, TestInteraction};
    use mensung_domain::{EvidenceLevel, Severity};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_c() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
    }

    fn type_str(app: &mut App, text: &str) {
        for c in text.chars() {
            app.handle_key(key(KeyCode::Char(c)));
        }
    }

    fn test_bytes() -> Vec<u8> {
        build_men_file(
            vec![
                TestDrug::plain(0, "Aspirin"),
                TestDrug::plain(1, "Warfarin"),
                TestDrug::plain(2, "Paracetamol"),
            ],
            &[TestInteraction::simple(
                0,
                0,
                1,
                Severity::Contraindicated,
                EvidenceLevel::Established,
                "test",
                "Increased bleeding and hemorrhage probability.",
            )],
            &[],
        )
    }

    #[test]
    fn starts_on_the_input_screen_with_the_first_field_focused() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let app = App::new(&db);
        assert!(matches!(app.screen(), Screen::Input));
        assert_eq!(app.focused(), 0);
    }

    #[test]
    fn typing_appends_to_the_focused_field() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        type_str(&mut app, "Asp");
        assert_eq!(app.inputs()[0], "Asp");
        assert_eq!(app.inputs()[1], "");
    }

    #[test]
    fn tab_switches_the_focused_field() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focused(), 1);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focused(), 0);
    }

    #[test]
    fn backspace_removes_the_last_character() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        type_str(&mut app, "Asp");
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.inputs()[0], "As");
    }

    #[test]
    fn esc_on_the_input_screen_quits() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        app.handle_key(key(KeyCode::Esc));
        assert!(app.should_quit());
    }

    #[test]
    fn ctrl_c_quits_from_any_screen() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        app.handle_key(ctrl_c());
        assert!(app.should_quit());
    }

    #[test]
    fn enter_with_an_empty_field_does_not_submit() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        type_str(&mut app, "Aspirin");
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(app.screen(), Screen::Input));
    }

    #[test]
    fn two_exact_names_go_straight_to_results() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        type_str(&mut app, "Aspirin");
        app.handle_key(key(KeyCode::Tab));
        type_str(&mut app, "Warfarin");
        app.handle_key(key(KeyCode::Enter));

        match app.screen() {
            Screen::Results { interactions } => {
                assert_eq!(interactions.len(), 1);
                assert_eq!(interactions[0].severity(), Severity::Contraindicated);
            }
            other => panic!("expected Results, got {other:?}"),
        }
    }

    #[test]
    fn a_typo_offers_candidates_and_never_auto_corrects() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        type_str(&mut app, "Aspirn");
        app.handle_key(key(KeyCode::Tab));
        type_str(&mut app, "Warfarin");
        app.handle_key(key(KeyCode::Enter));

        match app.screen() {
            Screen::Candidates {
                field, candidates, ..
            } => {
                assert_eq!(*field, 0);
                assert!(candidates.iter().any(|c| c.drug().name() == "Aspirin"));
            }
            _ => panic!("expected Candidates"),
        }
    }

    #[test]
    fn confirming_a_candidate_fills_the_field_and_continues_resolution() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        type_str(&mut app, "Aspirn");
        app.handle_key(key(KeyCode::Tab));
        type_str(&mut app, "Warfarin");
        app.handle_key(key(KeyCode::Enter));

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.inputs()[0], "Aspirin");
        match app.screen() {
            Screen::Results { interactions } => assert_eq!(interactions.len(), 1),
            _ => panic!("expected Results after confirming the only candidate match"),
        }
    }

    #[test]
    fn no_similar_name_shows_no_match() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        type_str(&mut app, "Zzzzzzzzzzzz");
        app.handle_key(key(KeyCode::Tab));
        type_str(&mut app, "Warfarin");
        app.handle_key(key(KeyCode::Enter));

        assert!(matches!(app.screen(), Screen::NoMatch { field: 0 }));
    }

    #[test]
    fn dismissing_a_result_screen_returns_to_input() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        type_str(&mut app, "Aspirin");
        app.handle_key(key(KeyCode::Tab));
        type_str(&mut app, "Warfarin");
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Enter));

        assert!(matches!(app.screen(), Screen::Input));
    }

    #[test]
    fn dismissing_a_result_screen_clears_the_fields_for_the_next_lookup() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        type_str(&mut app, "Aspirin");
        app.handle_key(key(KeyCode::Tab));
        type_str(&mut app, "Warfarin");
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.inputs()[0], "");
        assert_eq!(app.inputs()[1], "");
        assert_eq!(app.focused(), 0);

        type_str(&mut app, "Paracetamol");
        assert_eq!(app.inputs()[0], "Paracetamol");
    }

    #[test]
    fn dismissing_a_no_match_screen_also_clears_the_fields() {
        let bytes = test_bytes();
        let db = Database::open(&bytes).unwrap();
        let mut app = App::new(&db);
        type_str(&mut app, "Zzzzzzzzzzzz");
        app.handle_key(key(KeyCode::Tab));
        type_str(&mut app, "Warfarin");
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.inputs()[0], "");
        assert_eq!(app.inputs()[1], "");
    }
}
