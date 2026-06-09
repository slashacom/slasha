
const fs = require('fs');

const version = process.env.SLASHA_VERSION?.replace('v', '');

if (!version) {
    throw new Error('SLASHA_VERSION environment variable not set');
}
const path = require('path');

const files = [
    path.join(__dirname, '../crates/slasha-cli/Cargo.toml')
];

for (const file of files) {
    if (fs.existsSync(file)) {
        let content = fs.readFileSync(file, 'utf8');
        content = content.replace(/^version = ".*?"/m, `version = "${version}"`);
        fs.writeFileSync(file, content, 'utf8');
    }
}
