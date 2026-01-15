const esbuild = require('esbuild');
const path = require('path');

async function build() {
    console.log('Building editor bundle...');
    try {
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
    } catch (e) {
        console.error('Build failed:', e);
        process.exit(1);
    }
}

build();
