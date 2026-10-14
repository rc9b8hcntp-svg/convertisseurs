/* Copie les bibliothèques et les polices depuis node_modules vers src/vendor,
   pour que l'application fonctionne sans internet. Lancé automatiquement avant chaque build. */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const nm = (...p) => path.join(root, 'node_modules', ...p);
const out = (...p) => path.join(root, 'src', 'vendor', ...p);

function fail(msg) { console.error('\nERREUR vendor : ' + msg + '\n'); process.exit(1); }
function pick(label, candidates) {
  for (const c of candidates) if (fs.existsSync(c) && fs.statSync(c).size > 1000) return c;
  fail('fichier introuvable pour ' + label + ' (cherché : ' + candidates.join(', ') + '). Lance d’abord « npm install ».');
}

fs.rmSync(out(), { recursive: true, force: true });
fs.mkdirSync(out('fonts'), { recursive: true });

/* ---- Bibliothèques JavaScript ---- */
const LIBS = [
  ['pdf.min.js',          [nm('pdfjs-dist', 'build', 'pdf.min.js')]],
  ['pdf.worker.min.js',   [nm('pdfjs-dist', 'build', 'pdf.worker.min.js')]],
  ['jspdf.umd.min.js',    [nm('jspdf', 'dist', 'jspdf.umd.min.js')]],
  ['pdf-lib.min.js',      [nm('pdf-lib', 'dist', 'pdf-lib.min.js')]],
  ['jszip.min.js',        [nm('jszip', 'dist', 'jszip.min.js')]],
  ['mammoth.browser.min.js', [nm('mammoth', 'mammoth.browser.min.js')]],
  ['xlsx.full.min.js',    [nm('xlsx', 'dist', 'xlsx.full.min.js')]],
  ['lame.min.js',         [nm('lamejs', 'lame.min.js'), nm('lamejs', 'lame.all.js'), nm('lamejs', 'dist', 'lame.min.js')]]
];
for (const [name, cands] of LIBS) {
  fs.copyFileSync(pick(name, cands), out(name));
  console.log('ok  ' + name);
}

/* ---- Polices (latin et latin-ext seulement) ---- */
function copyFont(pkg, dest, cssList, renameFamily) {
  const src = nm(...pkg.split('/'));
  if (!fs.existsSync(src)) fail('police absente : ' + pkg + '. Lance d’abord « npm install ».');
  const d = out('fonts', dest);
  fs.mkdirSync(path.join(d, 'files'), { recursive: true });
  for (const css of cssList) {
    let txt = fs.readFileSync(pick(pkg + '/' + css, [path.join(src, css)]), 'utf8');
    if (renameFamily) txt = txt.split(renameFamily[0]).join(renameFamily[1]);
    fs.writeFileSync(path.join(d, css), txt);
  }
  for (const f of fs.readdirSync(path.join(src, 'files'))) {
    if (/latin/.test(f) && !/italic/.test(f)) fs.copyFileSync(path.join(src, 'files', f), path.join(d, 'files', f));
  }
  console.log('ok  polices ' + dest);
}
copyFont('@fontsource-variable/bricolage-grotesque', 'bricolage-grotesque', ['index.css'], ['Bricolage Grotesque Variable', 'Bricolage Grotesque']);
copyFont('@fontsource/ibm-plex-sans', 'ibm-plex-sans', ['400.css', '500.css', '600.css']);
copyFont('@fontsource/ibm-plex-mono', 'ibm-plex-mono', ['400.css', '500.css']);

/* ---- Vérification : tout ce que index.html demande existe ---- */
const html = fs.readFileSync(path.join(root, 'src', 'index.html'), 'utf8');
const used = [...new Set(html.match(/vendor\/[A-Za-z0-9_.\/-]+/g) || [])];
const missing = used.filter((u) => !fs.existsSync(path.join(root, 'src', u)));
if (missing.length) fail('index.html demande des fichiers absents : ' + missing.join(', '));
console.log('\nTout est prêt (' + used.length + ' fichiers vérifiés).');
