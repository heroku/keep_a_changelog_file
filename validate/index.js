const core = require('@actions/core');
const glob = require('@actions/glob');

main().catch(e => core.setFailed(e.message))

async function main() {
    const globber = await glob.create(core.getInput('changelogs'))
    const changelogs = await globber.glob()
    console.log(`Changelogs: ${changelogs.join("\n")}`)
}
