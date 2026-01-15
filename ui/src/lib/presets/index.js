// Auto-import all preset JSON files from this folder
// To add a new preset, just create a new .json file here - no code changes needed!

// Vite glob import - automatically finds all .json files
const presetModules = import.meta.glob('./*.json', { eager: true });

// Convert to array and sort (recommended first, then alphabetically)
export const builtInPresets = Object.values(presetModules)
  .map(module => module.default)
  .sort((a, b) => {
    // Recommended presets first
    if (a.recommended && !b.recommended) return -1;
    if (!a.recommended && b.recommended) return 1;
    // Then sort alphabetically by name
    return a.name.localeCompare(b.name);
  });

export default builtInPresets;
