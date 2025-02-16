import init, {get_errors} from "./demo_parser.js"
import {getMultilineInput, setFailed, debug, error} from "@actions/core"
import {create as globCreate} from "@actions/glob"
import {readFile} from 'node:fs/promises'

try {
    await init()
    for (const changelogInput of getMultilineInput('changelogs', {required: true})) {
        const changelogs = globCreate(changelogInput, {matchDirectories: false})
        for await (const changelog of changelogs.globGenerator()) {
            debug(`Processing changelog: ${changelog}`)
            const changelogContent = await readFile(changelog, 'utf8')
            for (const diagnostic in get_errors(changelogContent.toString())) {
                error(diagnostic.message, {
                    title: diagnostic.message,
                    file: changelog,
                    startLine: diagnostic.start.line,
                    endLine: diagnostic.end.line,
                    startColumn: diagnostic.start.column,
                    endColumn: diagnostic.end.column,
                })
            }
        }
    }
} catch (error) {
    setFailed(error.message);
}
