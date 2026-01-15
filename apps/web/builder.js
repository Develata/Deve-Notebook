const esbuild = require('esbuild');
const path = require('path');
const fs = require('fs');

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
            // Recursive copy
            await fs.promises.cp(asset.src, asset.dest, { recursive: true });
            console.log(`Copied: ${asset.src} -> ${asset.dest}`);
        } else {
            console.warn(`Warning: Asset source not found: ${asset.src}`);
        }
    }
}

async function build() {
    console.log('Building editor bundle...');
    try {
        // 1. Build JS Bundle
        await esbuild.build({
            entryPoints: ['js/editor_adapter.js'],
            bundle: true,
            outfile: 'js/editor.bundle.js',
            format: 'esm',
            minify: true,
            sourcemap: true,
            target: ['es2020'],
            external: [], // Bundle everything, including mermaid
        });
        console.log('Build complete: js/editor.bundle.js');

        // 2. Copy Static Assets (e.g. KaTeX)
        await copyAssets();
        
    } catch (e) {
        console.error('Build failed:', e);
        process.exit(1);
    }
}

build();
