const esbuild = require('esbuild');
const path = require('path');
const fs = require('fs');

async function writeIfChanged(filePath, content) {
    const next = Buffer.isBuffer(content) ? content : Buffer.from(content);
    const prev = await fs.promises.readFile(filePath).catch(() => null);
    if (prev && Buffer.compare(prev, next) === 0) {
        return false;
    }
    await fs.promises.mkdir(path.dirname(filePath), { recursive: true });
    await fs.promises.writeFile(filePath, next);
    return true;
}

async function syncDir(srcDir, destDir) {
    const entries = await fs.promises.readdir(srcDir, { withFileTypes: true });
    await fs.promises.mkdir(destDir, { recursive: true });
    let changed = false;
    for (const entry of entries) {
        const src = path.join(srcDir, entry.name);
        const dest = path.join(destDir, entry.name);
        if (entry.isDirectory()) {
            changed = (await syncDir(src, dest)) || changed;
            continue;
        }
        const content = await fs.promises.readFile(src);
        changed = (await writeIfChanged(dest, content)) || changed;
    }
    return changed;
}

async function copyAssets() {
    console.log('Copying static assets...');
    
    // Config: Source -> Dest
    const assets = [
        {
            src: path.join(__dirname, 'node_modules/katex/dist'),
            dest: path.join(__dirname, 'public/katex')
        }
    ];

    for (const asset of assets) {
        if (fs.existsSync(asset.src)) {
            const changed = await syncDir(asset.src, asset.dest);
            console.log(
                changed
                    ? `Copied: ${asset.src} -> ${asset.dest}`
                    : `Skipped unchanged: ${asset.dest}`
            );
        } else {
            console.warn(`Warning: Asset source not found: ${asset.src}`);
        }
    }
}

async function build() {
    console.log('Building editor bundle...');
    try {
        const result = await esbuild.build({
            entryPoints: ['js/editor_adapter.js'],
            bundle: true,
            outfile: 'js/editor.bundle.js',
            format: 'esm',
            minify: true,
            sourcemap: true,
            write: false,
            target: ['es2020'],
            external: [],
        });
        let bundleChanged = false;
        for (const file of result.outputFiles || []) {
            bundleChanged = (await writeIfChanged(file.path, file.contents)) || bundleChanged;
        }
        console.log(
            bundleChanged
                ? 'Build complete: js/editor.bundle.js'
                : 'Build unchanged: js/editor.bundle.js'
        );

        await copyAssets();
        
    } catch (e) {
        console.error('Build failed:', e);
        process.exit(1);
    }
}

build();
