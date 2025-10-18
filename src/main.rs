use std::process::Command;

use rayon::{iter::ParallelIterator as _, str::ParallelString as _};

fn main() {
    let bookmarks = Command::new("jj")
        .args([
            "log",
            "--reversed",
            "--ignore-working-copy",
            "--no-graph",
            "--color",
            "never",
            "-r",
            "coalesce(heads(::@ & bookmarks() & mutable()), trunk())",
            "-T",
            r#"bookmarks.map(|b| b.name()).join("\0") ++ "\0""#,
            "--config",
            "colors.none='default'",
        ])
        .output()
        .unwrap()
        .stdout;

    let bookmarks = String::from_utf8(bookmarks).unwrap();
    let distances = bookmarks.par_split('\0').filter_map(|bookmark| {
        if bookmark.is_empty() {
            return None;
        }

        let distance: String = Command::new("jj")
            .args([
                "log",
                "--reversed",
                "--ignore-working-copy",
                "--no-graph",
                "--color",
                "always",
                "-T",
            ])
            .arg(format!(
                "if(bookmarks.any(|b| b.name() == '{bookmark}'),\
                    label('bookmark', '{bookmark}'),\
                    '+')"
            ))
            .arg("-r")
            .arg(format!("{bookmark}::@"))
            .output()
            .unwrap()
            .stdout
            .try_into()
            .unwrap();

        Some(format!("{distance}"))
    });

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
