## Rust URL Shortener

This project is a URL Shortener that I wrote in Rust on the back end, with some simple HTML, CSS, and HTMX (though I'm sure JS will appear at some point). 

### Build
`cargo build --release` (release flag optional) should give you a binary that can be run in place. NOTE: 
This binary cannot be moved because it does rely on the `html` and `templates` folders. At some point, I 
plan on implementing a more proper install method, but there are other basic features that I want to focus
on first. 

### Running
Running the binary after build should work just fine as long as you stay in the project's root directory,
though `cargo run --release` is recommended.
This will create a default `config.toml` that you must update to match your environment. 

### To-Do
The following are items that I still need to get working:
- [ ] Login System
- [ ] URL Management by User
- [ ] User Permissions
- [ ] Admin Area
