// Render the checked-in Markdown chapters into one self-contained offline book.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, resolve, relative } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const requested = process.argv.indexOf('--output');
const output = requested < 0 ? resolve(root, 'docs/guide.html') : resolve(process.argv[requested + 1]);
const require = createRequire(import.meta.url);
let markedPath;
try {
  markedPath = require.resolve('marked', { paths: [root, resolve(root, '.doc-tools'), process.env.CODEX_PRIMARY_RUNTIME_NODE_MODULES || root] });
} catch {
  throw new Error('Install the documentation renderer first: npm install --prefix .doc-tools --no-save --no-package-lock marked');
}
const { marked } = await import(pathToFileURL(markedPath).href);
const chapters = [
  ['docs/README.md', 'overview', 'Handbook overview'],
  ['docs/getting-started.md', 'getting-started', 'Getting started'],
  ['docs/language.md', 'language', 'Language reference'],
  ['docs/standard-library.md', 'library', 'Standard library'],
  ['docs/graphics-and-audio.md', 'media', 'Graphics, input, and audio'],
  ['docs/scheduling.md', 'scheduling', 'Ticks, frames, and timers'],
  ['docs/cli.md', 'cli', 'Command-line reference'],
  ['docs/recipes.md', 'recipes', 'Recipes'],
  ['docs/troubleshooting.md', 'troubleshooting', 'Troubleshooting'],
  ['docs/windows-installation.md', 'windows', 'Windows installation'],
  ['README.md', 'project', 'Building and project README'],
  ['docs/architecture.md', 'architecture', 'Implementation map'],
  ['docs/validation.md', 'validation', 'Validation record'],
  ['THIRD_PARTY.md', 'third-party', 'Third-party components'],
];
const chapterByPath = new Map(chapters.map(([path, id]) => [resolve(root, path), id]));
const escape = text => text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
const slug = text => text.toLowerCase().replace(/<[^>]*>/g, '').replace(/&[^;]*;/g, '').replace(/[^\p{L}\p{N}_\s-]/gu, '').trim().replace(/\s/g, '-');
const sections = [];
for (const [path, chapter, title] of chapters) {
  const markdown = await readFile(resolve(root, path), 'utf8');
  let html = marked.parse(markdown, { gfm: true, async: false });
  const counts = new Map();
  html = html.replace(/<h([1-6])>([\s\S]*?)<\/h\1>/g, (_, level, body) => {
    const base = slug(body);
    const occurrence = counts.get(base) || 0;
    counts.set(base, occurrence + 1);
    const anchor = `${chapter}-${base}${occurrence ? '-' + occurrence : ''}`;
    return `<h${level} id="${anchor}">${body}</h${level}>`;
  });
  html = html.replace(/href="([^"]*)"/g, (_, href) => {
    if (/^[a-z][a-z0-9+.-]*:/i.test(href)) return `href="${href}"`;
    const [part, anchor] = href.split('#', 2);
    const destination = part ? resolve(root, dirname(path), decodeURIComponent(part)) : resolve(root, path);
    const target = chapterByPath.get(destination);
    if (target) return `href="#${target}${anchor ? '-' + anchor : ''}"`;
    // License text remains in the project download, never as a broken local link
    // in the standalone handbook.
    return `data-project-file="${escape(relative(root, destination))}" title="Included in the project ZIP"`;
  });
  html = html.replaceAll('<table>', '<div class="table-scroll"><table>').replaceAll('</table>', '</table></div>');
  sections.push(`<section class="chapter" id="${chapter}" aria-label="${escape(title)}">${html}<p class="chapter-end"><a href="#contents">Back to contents</a></p></section>`);
}
const nav = chapters.map(([, id, title]) => `<li><a href="#${id}">${escape(title)}</a></li>`).join('\n');
const page = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'">
<title>Indium Handbook — 0.1.0-alpha.1</title>
<style>
:root{color-scheme:light;--ink:#202437;--muted:#5d6478;--accent:#5544bc;--line:#e1e4ee;--paper:#fff;--code:#f2f4fa}
*{box-sizing:border-box}html{scroll-behavior:smooth;scroll-padding-top:1.5rem}body{margin:0;background:#f7f8fc;color:var(--ink);font:16px/1.7 system-ui,-apple-system,"Segoe UI",sans-serif}
a{color:var(--accent);text-decoration-thickness:1px;text-underline-offset:3px}a:hover{text-decoration-thickness:2px}aside{position:fixed;inset:0 auto 0 0;width:280px;padding:2rem 1.5rem;background:var(--paper);border-right:1px solid var(--line);overflow:auto}
.wordmark{font-size:1.7rem;font-weight:750;letter-spacing:-.045em}.edition{color:var(--muted);font-size:.82rem;margin:.2rem 0 1.2rem}.find{font-size:.82rem;color:var(--muted);padding:.7rem;background:var(--code);border-radius:8px}nav ol{padding-left:1.2rem}nav li{margin:.55rem 0;line-height:1.45}nav a{font-size:.9rem;text-decoration:none}
main{max-width:1130px;margin:0 auto 0 280px;padding:3rem 3.5rem 6rem}.cover{padding-bottom:2rem;border-bottom:2px solid var(--line)}.eyebrow{text-transform:uppercase;letter-spacing:.12em;font-size:.75rem;color:var(--accent);font-weight:700}.cover h1{font-size:3.4rem;line-height:1.1;letter-spacing:-.045em;margin:.5rem 0 1rem}.lede{font-size:1.15rem;color:var(--muted);max-width:65ch}.cover small{color:var(--muted)}
.chapter{margin-top:4rem;padding:0 0 1rem;scroll-margin-top:2rem}.chapter>h1{font-size:2rem;line-height:1.2;border-bottom:1px solid var(--line);padding-bottom:1rem;margin:0 0 1.7rem;letter-spacing:-.025em}h2{font-size:1.35rem;line-height:1.3;margin:2.2rem 0 .9rem}h3{font-size:1.12rem;margin-top:1.7rem}p,ul,ol{max-width:80ch}li{padding-left:.15rem}p{margin:.85rem 0}strong{font-weight:650}
code{font-family:ui-monospace,"Cascadia Code",Consolas,monospace;font-size:.88em;background:var(--code);border-radius:4px;padding:.12rem .3rem;overflow-wrap:anywhere}pre{background:#1e2233;color:#eef0fb;border:1px solid #30364e;border-radius:10px;padding:1.15rem 1.3rem;overflow:auto;line-height:1.6;margin:1.2rem 0}pre code{font-size:.86rem;padding:0;background:transparent;overflow-wrap:normal;white-space:pre}
.table-scroll{overflow-x:auto;margin:1.2rem 0;border:1px solid var(--line);border-radius:8px}table{width:100%;border-collapse:collapse;background:var(--paper);font-size:.9rem;line-height:1.55}th,td{text-align:left;padding:.8rem .9rem;border-bottom:1px solid var(--line);vertical-align:top}th{background:#eceff8;font-weight:650}tr:last-child td{border-bottom:0}td:first-child{min-width:130px}blockquote{margin-left:0;border-left:3px solid var(--accent);padding:.2rem 1rem;color:var(--muted)}.chapter-end{margin-top:2rem;font-size:.85rem}::selection{background:#dcd6ff}
@media(min-width:1500px){main{margin-left:calc(280px + (100vw - 1500px)/2)}}@media(max-width:950px){aside{position:static;width:auto;border-right:0;border-bottom:1px solid var(--line)}nav ol{columns:2;column-gap:2rem}nav li{break-inside:avoid}main{margin:0;padding:2rem 1.3rem 4rem}.cover h1{font-size:2.5rem}.chapter{margin-top:3rem}}@media(max-width:540px){nav ol{columns:1}body{font-size:15px}th,td{padding:.65rem}pre{padding:.9rem}.chapter>h1{font-size:1.65rem}}
@media print{body{background:#fff;font-size:10pt}aside,.chapter-end{display:none}main{max-width:none;margin:0;padding:0}.cover{break-after:page}.chapter{margin-top:0;break-before:page}h1,h2,h3{break-after:avoid}pre{white-space:pre-wrap;background:#f3f4f8;color:#111;break-inside:avoid}pre code{white-space:pre-wrap;font-size:8pt}.table-scroll{overflow:visible}table{font-size:8.5pt}a{color:#222;text-decoration:none}tr{break-inside:avoid}}
</style></head><body>
<aside id="contents"><div class="wordmark">Indium</div><div class="edition">HANDBOOK · 0.1.0-alpha.1</div><p class="find">One offline page. Use Ctrl+F to find a function, feature, or error message.</p><nav aria-label="Chapters"><ol>${nav}</ol></nav></aside>
<main><header class="cover"><div class="eyebrow">The implemented language</div><h1>Indium handbook</h1><p class="lede">From your first .in file to functions, files, OpenGL graphics, audio, and independent simulation ticks.</p><p>Examples, exact API behavior, practical limits, and troubleshooting for the current pre-alpha.</p><small>Source: the Markdown chapters included in the project ZIP. Updated 29 October 2026. External links open only when you choose them; this handbook needs no network connection.</small></header>
${sections.join('\n')}
</main></body></html>`;
const ids = [...page.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]);
if (new Set(ids).size !== ids.length) throw new Error('Duplicate handbook anchor IDs');
const targets = new Set(ids);
for (const [, target] of page.matchAll(/href="#([^"]+)"/g)) {
  if (!targets.has(target)) throw new Error('Broken handbook anchor: ' + target);
}
await mkdir(dirname(output), { recursive: true });
await writeFile(output, page, 'utf8');
console.log(`Built ${output}: ${chapters.length} chapters, ${Buffer.byteLength(page)} bytes; internal links checked`);
