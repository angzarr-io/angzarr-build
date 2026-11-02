# Messages for this crate's diagnostics. Each id is a diagnostic's code
# without the crate's name, which the build prefixes
# (angzarr_diagnostic::build); the variables are its args.

no-out-dir = OUT_DIR is unset: run this from a build script
io = { $path }: { $detail }
protoc = protoc failed: { $detail }
prost = prost-build failed: { $detail }
codegen = { $error }
buf = buf generate failed: { $detail }
uv = uv failed: { $detail }
interface-missing = { $path }, a policy's interface, is not there: its adapter translates no refusal; scaffold it, or move it back
