# TODO: Ship the Python documentation with the app

Once [the Python documentation](todo_python_doc.md) exists, it should travel
with the application instead of living only on a website:

- Put it in the **flatpak** and in the **Windows installer**.
- Inside the flatpak, a small local HTTP server could serve the built HTML so
  the user can browse it in their normal browser.
- **Nix** support is to be discussed.
