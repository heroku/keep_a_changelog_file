const core = require('@actions/core')
const glob = require('@actions/glob')
const {readFile} = require('node:fs/promises')
const {changelog_js} = require('../pkg')

main().catch(e => core.setFailed(e.message))

async function main() {
    const globber = await glob.create(core.getInput('changelogs'))
    const changelogs = await globber.glob()
    console.log(`Changelogs: ${changelogs.join("\n")}`)

    const files = {}
    for await (const changelog of changelogs) {
        files[changelog] = await readFile(changelog, 'utf-8')
    }

    const results = JSON.parse(changelog_js(JSON.stringify({files})))

    console.log(`Results: ${Object.entries(results).map(([file, valid]) => `${file}: ${valid}`).join('\n')}`)
}
