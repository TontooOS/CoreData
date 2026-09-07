use coredata::{FetchRequest, PersistentContainer, Predicate, PredicateOperator, SortDescriptor, StoreType};

fn main() -> coredata::Result<()> {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("TONTOO_PREFERENCES_ROOT", dir.path().to_string_lossy().to_string());
    std::env::set_var("TONTOO_COREDATA_ALLOW_FOREIGN", "1");
    let keyfile = dir.path().join("keyfile");
    std::env::set_var("TONTOO_COREDATA_KEY_FILE", keyfile.to_string_lossy().to_string());

    let bundle = "org.example.sqlite".to_string();
    let mut container = PersistentContainer::new_with_bundle(bundle.clone(), StoreType::SQLite)?;
    {
        let mut ctx = container.view_context();
        for (name, age) in [("Alice", 30), ("Bob", 20), ("Charlie", 25)] {
            let mut p = ctx.create("Person");
            p.set("name", name);
            p.set("age", age);
            ctx.save_object(p)?;
        }
        ctx.save()?;
        println!("saved 3 persons");
    }
    let mut container2 = PersistentContainer::new_with_bundle(bundle, StoreType::SQLite)?;
    let ctx2 = container2.view_context();
    let req = FetchRequest::new("Person")
        .predicate(Predicate::new("age", PredicateOperator::GreaterThan, "22"))
        .sorted_by(SortDescriptor::new("age", false));
    let res = ctx2.fetch(req)?;
    println!("fetch age>22 sorted desc: {} results", res.len());
    for o in res {
        println!(" - {} age={}", o.get_str("name").unwrap(), o.get_i64("age").unwrap());
    }
    Ok(())
}
