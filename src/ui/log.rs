use std::rc::Rc;

use yew::Reducible;

use crate::discord::LogLine;

const MAX_LINES: usize = 300;

#[derive(Default, PartialEq)]
pub struct LogModel {
    pub lines: Vec<Rc<LogLine>>,
}

pub enum LogAction {
    Push(LogLine),
    Clear,
}

impl Reducible for LogModel {
    type Action = LogAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut lines = self.lines.clone();
        match action {
            LogAction::Push(line) => {
                lines.push(Rc::new(line));
                if lines.len() > MAX_LINES {
                    lines.remove(0);
                }
            }
            LogAction::Clear => lines.clear(),
        }
        Rc::new(Self { lines })
    }
}
