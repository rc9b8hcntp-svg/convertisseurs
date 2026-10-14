/* Met le numéro de version (ex. 1.2.3) dans tauri.conf.json, Cargo.toml et package.json.
   Lancé automatiquement par GitHub quand tu publies un tag « v1.2.3 ». */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const v = String(process.argv[2] || '').replace(/^v/, '').trim();
if (!/^\d+\.\d+\.\d+$/.test(v)) {
  console.error('Version invalide : « ' + v + ' ». Utilise un tag comme v1.2.3 (trois nombres).');
  process.exit(1);
}
const [maj, min, pat] = v.split('.').map(Number);
if (maj > 255 || min > 255 || pat > 65535) {
  console.error('Version trop grande pour un MSI (max 255.255.65535).');
  process.exit(1);
}

function edit(file, fn) {
  const f = path.join(root, file);
  fs.writeFileSync(f, fn(fs.readFileSync(f, 'utf8')));
}
edit('src-tauri/tauri.conf.json', (s) => s.replace(/("version"\s*:\s*")[^"]*(")/, `$1${v}$2`));
edit('package.json', (s) => s.replace(/("version"\s*:\s*")[^"]*(")/, `$1${v}$2`));
edit('src-tauri/Cargo.toml', (s) => s.replace(/^version\s*=\s*"[^"]*"/m, `version = "${v}"`));
console.log('Version mise à ' + v);
