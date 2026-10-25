use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: String,
    pub command: Vec<String>,
    pub icon: Option<PathBuf>,
    pub description: Option<String>,
    pub terminal: bool,
}

impl AsRef<str> for DesktopEntry {
    #[inline]
    fn as_ref(&self) -> &str {
        self.name.as_str()
    }
}

#[derive(Clone)]
pub struct DesktopEntries {
    pub items: Vec<DesktopEntry>,
}

pub fn load() -> DesktopEntries {
    let locales = freedesktop_desktop_entry::get_languages_from_env();
    let theme = freedesktop_icons::default_theme_gtk();

    let apps = freedesktop_desktop_entry::desktop_entries(&locales)
        .into_iter()
        .filter_map(|entry| desktop_entry(&entry, &locales, theme.as_deref()))
        .collect();

    DesktopEntries { items: apps }
}

fn desktop_entry(
    entry: &freedesktop_desktop_entry::DesktopEntry,
    locales: &[String],
    theme: Option<&str>,
) -> Option<DesktopEntry> {
    if entry.no_display() {
        return None;
    }

    let Some(name) = entry.name(locales) else {
        log::warn!(
            "Skipping desktop entry {}: missing Name",
            entry.path.display()
        );
        return None;
    };

    let command = match parse_command(entry, locales) {
        Ok(command) => command,
        Err(error) => {
            log::warn!("Skipping desktop entry {}: {error}", entry.path.display());
            return None;
        }
    };

    let description = entry
        .comment(locales)
        .map(|description| description.to_string());

    let icon = entry.icon().and_then(|icon| {
        let mut icon_builder = freedesktop_icons::lookup(icon)
            .with_size(64)
            .with_scale(1)
            .force_svg()
            .with_cache();

        if let Some(theme) = theme {
            icon_builder = icon_builder.with_theme(theme);
        }

        icon_builder.find()
    });

    Some(DesktopEntry {
        name: name.into_owned(),
        command,
        icon,
        description,
        terminal: entry.terminal(),
    })
}

fn parse_command(
    entry: &freedesktop_desktop_entry::DesktopEntry,
    locales: &[String],
) -> anyhow::Result<Vec<String>> {
    let Some(exec) = entry.exec().map(str::trim_start) else {
        anyhow::bail!("Exec key was not found");
    };
    if !exec.starts_with('"') {
        return Ok(entry.parse_exec_with_uris(&[], locales)?);
    }

    // Nix relocation can double the quotes around an already quoted absolute path.
    let delimiter = if exec.starts_with("\"\"/") {
        "\"\""
    } else {
        "\""
    };
    let remainder = exec
        .strip_prefix(delimiter)
        .ok_or_else(|| anyhow::anyhow!("missing opening quote"))?;
    let mut executable = String::new();
    let mut characters = remainder.char_indices();
    let mut arguments = None;
    while let Some((index, character)) = characters.next() {
        if character == '"' {
            let suffix = remainder
                .get(index..)
                .and_then(|suffix| suffix.strip_prefix(delimiter))
                .ok_or_else(|| anyhow::anyhow!("unmatched executable quote"))?;
            anyhow::ensure!(
                suffix.is_empty() || suffix.starts_with(char::is_whitespace),
                "unexpected text after executable quote"
            );
            arguments = Some(suffix);
            break;
        }
        if character == '\\' {
            let (_, escaped) = characters
                .next()
                .ok_or_else(|| anyhow::anyhow!("unfinished executable escape"))?;
            anyhow::ensure!(
                matches!(escaped, '"' | '\\' | '$' | '`'),
                "invalid executable escape"
            );
            executable.push(escaped);
        } else {
            executable.push(character);
        }
    }
    let arguments =
        arguments.ok_or_else(|| anyhow::anyhow!("unmatched executable quote"))?;
    anyhow::ensure!(
        !executable.is_empty() && !executable.contains('='),
        "invalid executable"
    );

    // Keep the dependency's field-code expansion, bypassing its assumption that
    // an opening quote encloses the entire command rather than the executable.
    let mut normalized = entry.clone();
    normalized.add_desktop_entry("Exec".into(), format!("ghost-executable{arguments}"));
    let mut command = normalized.parse_exec_with_uris(&[], locales)?;
    let first = command
        .first_mut()
        .ok_or_else(|| anyhow::anyhow!("empty command"))?;
    *first = executable;
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_continues_after_rejected_entries() -> anyhow::Result<()> {
        let locales = vec!["en".to_string()];
        let inputs = [
            "Name=Unmatched quote\nExec=\"/bin/example",
            "Name=Quoted executable\nExec=\"/bin/example\" --flag",
            "Name=Missing command",
            "Name=Empty command\nExec=",
            "Exec=/bin/example",
            "Name=Hidden\nExec=/bin/example\nNoDisplay=true",
            "Name=Working\nExec=/bin/example --flag %U\nComment=Example application\nTerminal=true",
        ];
        let entries = inputs
            .iter()
            .map(|input| {
                freedesktop_desktop_entry::DesktopEntry::from_str(
                    "/applications/example.desktop",
                    &format!("[Desktop Entry]\nType=Application\n{input}\n"),
                    Some(&locales),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let applications = entries
            .iter()
            .filter_map(|entry| desktop_entry(entry, &locales, None))
            .collect::<Vec<_>>();

        assert_eq!(applications.len(), 2);
        assert_eq!(
            applications
                .first()
                .map(|entry| entry.command.as_slice()),
            Some(["/bin/example".to_owned(), "--flag".to_owned()].as_slice())
        );
        let application = applications
            .last()
            .ok_or_else(|| anyhow::anyhow!("missing valid application"))?;
        assert_eq!(application.name, "Working");
        assert_eq!(application.command, ["/bin/example", "--flag"]);
        assert_eq!(
            application.description.as_deref(),
            Some("Example application")
        );
        assert!(application.terminal);
        Ok(())
    }
    #[test]
    fn quoted_executables() -> anyhow::Result<()> {
        let locales = vec!["en".to_owned()];
        for (exec, expected) in [
            (
                r#"""/nix/store/package/bin/gpclient"" launch-gui %u"#,
                vec!["/nix/store/package/bin/gpclient", "launch-gui"],
            ),
            (
                r#""/opt/Global Protect/gpclient" launch-gui %U"#,
                vec!["/opt/Global Protect/gpclient", "launch-gui"],
            ),
            (r#""/bin/example""#, vec!["/bin/example"]),
            (
                r#"/bin/example %c %k"#,
                vec![
                    "/bin/example",
                    "Global Protect",
                    "/applications/gpgui.desktop",
                ],
            ),
        ] {
            let entry = freedesktop_desktop_entry::DesktopEntry::from_str(
                "/applications/gpgui.desktop",
                &format!("[Desktop Entry]\nName=Global Protect\nExec={exec}\n"),
                Some(&locales),
            )?;
            assert_eq!(parse_command(&entry, &locales)?, expected, "{exec}");
        }
        for exec in [
            r#""/bin/example"suffix"#,
            r#"""/bin/example" --flag"#,
            r#""/bin/example"#,
            r#""" --flag"#,
        ] {
            let entry = freedesktop_desktop_entry::DesktopEntry::from_str(
                "/applications/gpgui.desktop",
                &format!("[Desktop Entry]\nName=Invalid\nExec={exec}\n"),
                Some(&locales),
            )?;
            assert!(desktop_entry(&entry, &locales, None).is_none(), "{exec}");
        }
        Ok(())
    }
}
