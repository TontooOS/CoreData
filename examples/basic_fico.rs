use coredata::{PersistentContainer, StoreType};

fn main() -> coredata::Result<()> {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("TONTOO_PREFERENCES_ROOT", dir.path().to_string_lossy().to_string());
    std::env::set_var("TONTOO_COREDATA_ALLOW_FOREIGN", "1");
    let keyfile = dir.path().join("keyfile");
    std::env::set_var("TONTOO_COREDATA_KEY_FILE", keyfile.to_string_lossy().to_string());

    let bundle = "org.example.basic".to_string();
    let mut container = PersistentContainer::new_with_bundle(bundle.clone(), StoreType::Fico)?;
    {
        let mut ctx = container.view_context();
        let mut note = ctx.create("Note");
        note.set("title", "Hello CoreData");
        note.set("done", false);
        note.set("priority", 1);
        ctx.save_object(note)?;
        ctx.save()?;
        println!("saved 1 Note");
    }
    // reload
    let mut container2 = PersistentContainer::new_with_bundle(bundle, StoreType::Fico)?;
    let ctx2 = container2.view_context();
    let notes = ctx2.fetch_all("Note")?;
    println!("fetched {} notes", notes.len());
    for n in notes {
        println!(" - {} done={:?}", n.get_str("title").unwrap_or("?"), n.get_bool("done"));
    }
    Ok(())
}
