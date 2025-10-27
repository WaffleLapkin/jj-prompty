use std::process::Command;

use rayon::{iter::ParallelIterator as _, str::ParallelString as _};

fn main() {
    let (future_bookmarks, prev_bookmarks) = rayon::join(
        || list_bookmarks("@:: & bookmarks()"),
        || list_bookmarks("coalesce(heads(::@ & bookmarks() & mutable()), trunk())"),
    );

    let (bookmarks, dir) = if !future_bookmarks.is_empty() {
        (future_bookmarks, Direction::Future)
    } else {
        (prev_bookmarks, Direction::Past)
    };

    let distances = bookmarks
        .par_split('\0')
        .filter_map(|bookmark| bookmark_distance(bookmark, dir));

    let statuses = rayon::iter::once(()).map(|()| {
        Command::new("jj")
            .args([
                "log",
                "--ignore-working-copy",
                "--no-graph",
                "--color",
                "always",
                "-r",
                "@",
                "-T",
                r#"separate(" ", if(conflict, label("conflict", "conflict")), if(empty, label("empty", "empty")), if(divergent, label("divergent", "divergent")), if(hidden, label("hidden", "hidden")))"#,
            ])
            .output()
            .unwrap()
            .stdout
            .try_into()
            .unwrap()
    }).filter(|s: &String| !s.is_empty());

    let output = distances
        .chain(statuses)
        .reduce_with(|a, b| format!("{a} {b}"));

    if let Some(output) = output {
        print!("({output})");
    }
}

fn list_bookmarks(r: &'static str) -> String {
    let bookmarks = Command::new("jj")
        .args([
            "log",
            "--reversed",
            "--ignore-working-copy",
            "--no-graph",
            "--color",
            "never",
            "-r",
            r,
            "-T",
            r#"bookmarks.map(|b| b.name()).join("\0") ++ "\0""#,
            "--config",
            "colors.none='default'",
        ])
        .output()
        .unwrap()
        .stdout;

    String::from_utf8(bookmarks).unwrap()
}

#[derive(Copy, Clone)]
enum Direction {
    Past,
    Future,
}

impl Direction {
    fn offset_char(self) -> char {
        match self {
            Direction::Past => '+',
            Direction::Future => '-',
        }
    }
}

fn bookmark_distance(bookmark: &str, dir: Direction) -> Option<String> {
    if bookmark.is_empty() {
        return None;
    }

    let offset_char = dir.offset_char();

    let mut cmd = Command::new("jj");

    cmd.args([
        "log",
        "--ignore-working-copy",
        "--no-graph",
        "--color",
        "always",
        "-T",
    ])
    .arg(format!(
        "if(bookmarks.any(|b| b.name() == '{bookmark}'),\
                    label('bookmark', '{bookmark}'),\
                    '{offset_char}')"
    ))
    .arg("-r")
    .arg(match dir {
        Direction::Past => format!("{bookmark}::@"),
        Direction::Future => format!("@::{bookmark}"),
    });

    if matches!(dir, Direction::Past) {
        cmd.arg("--reversed");
    }

    let distance: String = cmd.output().unwrap().stdout.try_into().unwrap();

    Some(format!("{distance}"))
}
