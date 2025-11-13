/-
Extract imports from a Lean source file using Lean's native parser.

This script is used by lemma's build system to accurately parse import statements
from Lean source files, avoiding the limitations of regex-based parsing.

Usage:
  lean --run extract_imports.lean <file.lean>

Output:
  One import per line (module names only)
-/

import Lean

open Lean

/-- Extract imports from a Lean source file -/
def extractImports (path : System.FilePath) : IO (Array String) := do
  -- Read the file contents
  let contents ← IO.FS.readFile path

  -- Parse imports using Lean's native parser
  -- This is the same parser used by the Lean compiler itself
  let header ← Lean.parseImports' contents path.toString

  -- Extract just the module names from the imports array
  -- ModuleHeader has a field `imports : Array Import`
  return header.imports.map (·.module.toString)

/-- Main entry point -/
def main (args : List String) : IO UInt32 := do
  match args with
  | [path] =>
    try
      let imports ← extractImports ⟨path⟩
      -- Print each import on its own line
      for imp in imports do
        IO.println imp
      return (0 : UInt32)
    catch e =>
      IO.eprintln s!"Error: {e}"
      return (1 : UInt32)
  | _ =>
    IO.eprintln "Usage: lean --run extract_imports.lean <file.lean>"
    return (1 : UInt32)
