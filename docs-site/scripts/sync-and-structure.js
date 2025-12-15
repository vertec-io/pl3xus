const fs = require('fs');
const path = require('path');
const matter = require('gray-matter');

const sourceDir = path.resolve(__dirname, '../../docs');
const contentDir = path.resolve(__dirname, '../content/docs');

// Define the order of top-level sections
// Define the order of sections for Core
const CORE_ORDER = [
    'index',
    'what-is-pl3xus',
    'comparisons',
    'getting-started',
    'installation',
    'guides',
    'examples',
    'architecture',
    'api',
    'reference',
    'releases',
    'migration'
];

const ROOT_ORDER = [
    'core',
    'sync',
    'client',
    'contributing'
];

// Map directory names to display titles
const TITLE_MAP = {
    'core': 'Core',
    'sync': 'Sync',
    'client': 'Client',
    'getting-started': 'Getting Started',
    'what-is-pl3xus': 'What is Pl3xus',
    'comparisons': 'Comparisons',
    'guides': 'Guides',
    'architecture': 'Architecture',
    'reference': 'API Reference',
    'examples': 'Examples',
    'installation': 'Installation',
    'introduction': 'Introduction'
};

function getTitleFromContent(content, filename) {
    // Try to find first h1
    const match = content.match(/^#\s+(.+)$/m);
    if (match) {
        return match[1].trim();
    }
    // Fallback to filename
    return filename.replace('.md', '').replace(/-/g, ' ').replace(/^\w/, c => c.toUpperCase());
}

function processFile(srcPath, destPath, filename) {
    const rawContent = fs.readFileSync(srcPath, 'utf8');
    const { data: existingFrontmatter, content } = matter(rawContent);

    // Determine title
    let title = existingFrontmatter.title;
    if (!title) {
        title = getTitleFromContent(rawContent, filename);
    }

    // Create new frontmatter
    const newFrontmatter = {
        ...existingFrontmatter,
        title: title
    };

    const newContent = matter.stringify(content, newFrontmatter);
    fs.writeFileSync(destPath, newContent);
}

function copyDir(src, dest) {
    if (!fs.existsSync(dest)) {
        fs.mkdirSync(dest, { recursive: true });
    }

    const entries = fs.readdirSync(src, { withFileTypes: true });
    const pages = [];

    for (const entry of entries) {
        const srcPath = path.join(src, entry.name);

        // Handle renaming README to index
        let destName = entry.name;
        if (entry.name.toLowerCase() === 'readme.md') {
            destName = 'index.md';
        }

        const destPath = path.join(dest, destName);

        if (entry.isDirectory()) {
            copyDir(srcPath, destPath);
            pages.push(entry.name);
        } else if (entry.isFile() && entry.name.endsWith('.md')) {
            processFile(srcPath, destPath, entry.name);

            if (destName !== 'index.md') {
                pages.push(destName.replace('.md', ''));
            }
        }
    }

    const isRoot = dest === contentDir;
    const dirName = path.basename(dest);
    const title = TITLE_MAP[dirName] || dirName.split('-').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');

    let meta = {
        title: isRoot ? "Documentation" : title
    };

    if (isRoot) {
        meta.pages = ROOT_ORDER;
    } else if (dirName === 'core') {
        meta.pages = CORE_ORDER;
    }

    // Ensure we don't choke if files exist that aren't in ORDER
    // (Fumadocs handles this gracefully usually, but good to be safe)

    fs.writeFileSync(path.join(dest, 'meta.json'), JSON.stringify(meta, null, 2));
}

// Clean target first
if (fs.existsSync(contentDir)) {
    fs.rmSync(contentDir, { recursive: true, force: true });
}

console.log(`Syncing docs from ${sourceDir} to ${contentDir}...`);

try {
    copyDir(sourceDir, contentDir);

    // Also copy root README.MD as the main index file (Introduction)
    const readmeSrc = path.resolve(__dirname, '../../README.MD');
    const indexDest = path.join(contentDir, 'index.md');
    if (fs.existsSync(readmeSrc)) {
        processFile(readmeSrc, indexDest, 'README.MD');
    }

    console.log('Docs sync complete.');
} catch (e) {
    console.error('Error syncing docs:', e);
    process.exit(1);
}
