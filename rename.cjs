const fs = require('fs');
const path = require('path');

function walkDir(dir, callback) {
    fs.readdirSync(dir).forEach(f => {
        let dirPath = path.join(dir, f);
        let isDirectory = fs.statSync(dirPath).isDirectory();
        if (isDirectory) {
            if (f !== 'node_modules' && f !== 'target' && !f.startsWith('.')) {
                walkDir(dirPath, callback);
            }
        } else {
            callback(dirPath);
        }
    });
}

const targetExtensions = ['.rs', '.json', '.html', '.css', '.js', '.md', '.toml'];

walkDir('d:\\Dev\\grammarlens', function (filePath) {
    if (!targetExtensions.some(ext => filePath.endsWith(ext))) return;

    // Skip lockfiles
    if (filePath.endsWith('package-lock.json') || filePath.endsWith('Cargo.lock')) return;

    let content = fs.readFileSync(filePath, 'utf8');
    let newContent = content
        .replace(/GrammarLens/g, 'TextLint')
        .replace(/grammarlens/g, 'textlint');

    if (content !== newContent) {
        fs.writeFileSync(filePath, newContent, 'utf8');
        console.log('Updated: ' + filePath);
    }
});
