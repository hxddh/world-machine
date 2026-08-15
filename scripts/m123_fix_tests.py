from pathlib import Path

path = Path('worlds/pocket-universe/src/lib.rs')
text = path.read_text()

old_background = '''        let briefing = after.briefing.as_ref().unwrap();
        assert_eq!(briefing.title, "While you were away");
        assert_eq!(briefing.items.len(), 3);
        assert!(briefing
            .items
            .iter()
            .all(|item| !item.detail.trim().is_empty()));
'''
new_background = '''        let briefing = after.briefing.as_ref().unwrap();
        assert_eq!(briefing.title, "While you were away");
        assert_eq!(
            briefing
                .items
                .iter()
                .filter(|item| item.selection.is_some())
                .count(),
            3,
            "the return digest should keep three selected history items"
        );
        assert!(briefing.items.iter().any(|item| {
            item.title == "Your turn · Relationship" && item.selection.is_none()
        }));
        assert!(briefing
            .items
            .iter()
            .all(|item| !item.detail.trim().is_empty()));
'''
if text.count(old_background) != 1:
    raise SystemExit(f'background return assertion match count was {text.count(old_background)}')
text = text.replace(old_background, new_background, 1)

old_agent = '''        assert_eq!(briefing.title, "While you were away");
        assert_eq!(briefing.items.len(), 3);
        assert!(briefing
            .items
            .iter()
            .all(|item| item.title != "agent decision recorded"));
'''
new_agent = '''        assert_eq!(briefing.title, "While you were away");
        assert_eq!(
            briefing
                .items
                .iter()
                .filter(|item| item.selection.is_some())
                .count(),
            3,
            "the return digest should stay bounded independently of the Compass"
        );
        assert!(briefing.items.iter().any(|item| {
            item.title == "Your turn · Relationship" && item.selection.is_none()
        }));
        assert!(briefing
            .items
            .iter()
            .all(|item| item.title != "agent decision recorded"));
'''
if text.count(old_agent) != 1:
    raise SystemExit(f'agent return assertion match count was {text.count(old_agent)}')
text = text.replace(old_agent, new_agent, 1)

path.write_text(text)
