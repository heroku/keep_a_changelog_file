const core = require('@actions/core');
const glob = require('@actions/glob');

try {
    const globber = await glob.create(core.getInput('changelogs'))
    const changelogs = await globber.glob()
    console.log(`Changelogs: ${changelogs.join("\n")}`)
} catch (error) {
    core.setFailed(error.message);
}
