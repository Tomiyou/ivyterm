use crate::terminal::Terminal;

#[derive(Debug)]
pub enum IvyError {
    TmuxSpawnError(String),
}

#[derive(PartialEq, Eq)]
pub struct SortedTerminal {
    pub id: u32,
    pub terminal: Terminal,
}

impl Ord for SortedTerminal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for SortedTerminal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Default)]
pub struct SortedTerminals {
    terminals: Vec<SortedTerminal>,
}

impl SortedTerminals {
    pub fn insert(&mut self, id: u32, terminal: &Terminal) -> usize {
        let terminal = SortedTerminal {
            id: id,
            terminal: terminal.clone(),
        };

        let insert_at = match self.terminals.binary_search(&terminal) {
            Ok(insert_at) | Err(insert_at) => insert_at,
        };
        self.terminals.insert(insert_at, terminal);
        insert_at
    }

    pub fn push(&mut self, id: u32, terminal: &Terminal) -> usize {
        let sorted_terminal = SortedTerminal {
            id: id,
            terminal: terminal.clone(),
        };

        if let Some(last) = self.terminals.last() {
            let cmp = sorted_terminal.cmp(last);
            if cmp == std::cmp::Ordering::Greater || cmp == std::cmp::Ordering::Equal {
                // The new element is greater than or equal to the current last element,
                // so we can simply push it onto the vec.
                self.terminals.push(sorted_terminal);
                self.terminals.len() - 1
            } else {
                // The new element is less than the last element in the container, so we
                // cannot simply push. We will fall back on the normal insert behavior.
                self.insert(id, terminal)
            }
        } else {
            // If there is no last element then the container must be empty, so we
            // can simply push the element and return its index, which must be 0.
            self.terminals.push(sorted_terminal);
            0
        }
    }

    pub fn remove(&mut self, id: u32) -> Option<Terminal> {
        match self
            .terminals
            .binary_search_by(|terminal| terminal.id.cmp(&id))
        {
            Ok(index) => Some(self.terminals.remove(index).terminal),
            Err(_) => None,
        }
    }

    pub fn get(&self, id: u32) -> Option<Terminal> {
        match self
            .terminals
            .binary_search_by(|terminal| terminal.id.cmp(&id))
        {
            Ok(index) => Some(self.terminals[index].terminal.clone()),
            Err(_) => None,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, SortedTerminal> {
        self.terminals.iter()
    }
}
