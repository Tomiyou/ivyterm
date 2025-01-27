use std::{
    cell::RefCell,
    process::{Command, Stdio},
};

use const_format::concatcp;

#[derive(thiserror::Error, Debug)]
pub enum IvyError {
    #[error("executing remote Tmux command failed")]
    TmuxSpawnFailed = 0,
}

pub enum TmuxError {
    EventChannelClosed,
    ExitEventReceived,
    SshClosed,
    ErrorParsingUTF8,
}

#[derive(Debug, PartialEq, Eq)]
pub struct WithId<T> {
    pub id: u32,
    pub terminal: T,
}

impl<T: PartialEq + Eq> PartialOrd for WithId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Eq> Ord for WithId<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

pub struct SortedVec<T> {
    terminals: Vec<WithId<T>>,
}

impl<T> Default for SortedVec<T> {
    fn default() -> Self {
        Self { terminals: vec![] }
    }
}

impl<T: Eq + Clone> SortedVec<T> {
    pub fn insert(&mut self, id: u32, terminal: &T) -> usize {
        let terminal = WithId {
            id: id,
            terminal: terminal.clone(),
        };

        let insert_at = match self.terminals.binary_search(&terminal) {
            Ok(insert_at) | Err(insert_at) => insert_at,
        };
        self.terminals.insert(insert_at, terminal);
        insert_at
    }

    pub fn push(&mut self, id: u32, terminal: &T) -> usize {
        let sorted_terminal = WithId {
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

    pub fn remove(&mut self, id: u32) -> Option<T> {
        match self
            .terminals
            .binary_search_by(|terminal| terminal.id.cmp(&id))
        {
            Ok(index) => Some(self.terminals.remove(index).terminal),
            Err(_) => None,
        }
    }

    pub fn get(&self, id: u32) -> Option<T> {
        match self
            .terminals
            .binary_search_by(|terminal| terminal.id.cmp(&id))
        {
            Ok(index) => Some(self.terminals[index].terminal.clone()),
            Err(_) => None,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, WithId<T>> {
        self.terminals.iter()
    }

    pub fn len(&self) -> usize {
        self.terminals.len()
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&WithId<T>) -> bool,
    {
        self.terminals.retain(f);
    }

    pub fn clear(&mut self) {
        self.terminals.clear();
    }
}

pub fn open_editor(path: &str, ssh_target: &Option<String>) {
    if path.is_empty() {
        return;
    }

    println!("Opening editor in path: {}", path);

    let mut command = Command::new("code");
    // Redirect stdin/stdout/stderr to /dev/null (we don't care about it)
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());

    // Check if this is a remote Tmux session and add this to the editor command
    if let Some(ssh_target) = ssh_target {
        // code --remote ssh-remote+SSH_TARGET PATH
        command.arg("--remote");
        let arg = format!("ssh-remote+{}", ssh_target);
        command.arg(&arg);
    }

    // Add path to the editor command
    command.arg(path);

    // Spawn editor
    match command.spawn() {
        Err(err) => {
            eprintln!("Error opening editor: {}", err);
        }
        _ => {}
    }
}

const USERCHARS: &str = "-[:alnum:]";
const USERCHARS_CLASS: &str = concatcp!("[", USERCHARS, "]");
const PASSCHARS_CLASS: &str = "[-[:alnum:]\\Q,?;.:/!%$^*&~\"#'\\E]";
const HOSTCHARS_CLASS: &str = "[-[:alnum:]]";
const HOST: &str = concatcp!(HOSTCHARS_CLASS, "+(\\.", HOSTCHARS_CLASS, "+)*");
const PORT: &str = "(?:\\:[[:digit:]]{1,5})?";
const PATHCHARS_CLASS: &str = "[-[:alnum:]\\Q_$.+!*,;:@&=?/~#%\\E]";
const PATHTERM_CLASS: &str = "[^\\Q]'.}>) \t\r\n,\"\\E]";
const SCHEME: &str = concatcp!(
    "(?:news:|telnet:|nntp:|file:\\/|https?:|ftps?:|sftp:|webcal:",
    "|irc:|sftp:|ldaps?:|nfs:|smb:|rsync:|ssh:|rlogin:|telnet:|git:",
    "|git\\+ssh:|bzr:|bzr\\+ssh:|svn:|svn\\+ssh:|hg:|mailto:|magnet:)"
);

const USERPASS: &str = concatcp!(USERCHARS_CLASS, "+(?:", PASSCHARS_CLASS, "+)?");
const URLPATH: &str = concatcp!(
    "(?:(/",
    PATHCHARS_CLASS,
    "+(?:[(]",
    PATHCHARS_CLASS,
    "*[)])*",
    PATHCHARS_CLASS,
    "*)*",
    PATHTERM_CLASS,
    ")?"
);

pub const URL_REGEX_STRINGS: [&str; 5] = [
    concatcp!(SCHEME, "//(?:", USERPASS, "\\@)?", HOST, PORT, URLPATH),
    concatcp!("(?:www|ftp)", HOSTCHARS_CLASS, "*\\.", HOST, PORT, URLPATH),
    concatcp!(
        "(?:callto:|h323:|sip:)",
        USERCHARS_CLASS,
        "[",
        USERCHARS,
        ".]*(?:",
        PORT,
        "/[a-z0-9]+)?\\@",
        HOST
    ),
    concatcp!(
        "(?:mailto:)?",
        USERCHARS_CLASS,
        "[",
        USERCHARS,
        ".]*\\@",
        HOSTCHARS_CLASS,
        "+\\.",
        HOST
    ),
    concatcp!("(?:news:|man:|info:)[[:alnum:]\\Q^_{|}~!\"#$%&'()*+,./;:=?`\\E]+"),
];

pub const PCRE2_MULTILINE: u32 = 0x00000400;

#[macro_export]
macro_rules! unwrap_or_return {
    ( $e:expr ) => {
        match $e {
            Some(x) => x,
            None => return,
        }
    };
}

#[inline]
pub fn borrow_clone<T>(cell: &RefCell<Option<T>>) -> T
where
    T: Clone,
{
    cell.borrow().clone().unwrap()
}
