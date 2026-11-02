Vendored from Waycrate revision d63ca8be6388846bdd3b2bb71fa1766fc28e5d91 (MIT).

Only runtime change: src/popup.rs creates and attaches a background-effect
object to popup surfaces, like layershell.rs already does. Cargo.toml resolves
upstream workspace metadata and dependencies explicitly. Remove this patch
once the pinned upstream revision includes popup background effects.
