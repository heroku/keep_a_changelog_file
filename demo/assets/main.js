import init, {get_errors, print_syntax_tree} from "./demo.js"
import {EditorState} from '@codemirror/state'
import {markdown} from '@codemirror/lang-markdown'
import {
    EditorView,
    lineNumbers,
    highlightActiveLineGutter,
    highlightSpecialChars,
    drawSelection,
    rectangularSelection,
    crosshairCursor,
    highlightActiveLine,
    keymap
} from '@codemirror/view';
import {defaultKeymap, history, historyKeymap} from "@codemirror/commands";
import {
    defaultHighlightStyle,
    indentOnInput,
    syntaxHighlighting
} from "@codemirror/language";
import {lintKeymap, linter, lintGutter} from "@codemirror/lint";
import spec from '../examples/spec.md?raw'
import nodejs from '../examples/nodejs.md?raw'
import minimal from '../examples/minimal.md?raw'
import missing_unreleased from '../examples/missing_unreleased.md?raw'
import release_with_no_changes from '../examples/release_with_no_changes.md?raw'
import changes_with_no_group from '../examples/changes_with_no_group.md?raw'
import changes_group_typo from '../examples/changes_group_typo.md?raw'


const EXAMPLES = [
    {
        id: 'keep-a-changelog',
        title: 'Keep a Changelog',
        text: spec
    },
    {
        id: 'nodejs-engine',
        title: "Node.js Engine CNB",
        text: nodejs
    },
    {
        id: 'minimal',
        title: 'Minimal',
        text: minimal
    },
    {
        id: 'empty',
        title: 'Empty',
        text: ''
    },
    {
        id: 'missing-unreleased',
        title: 'Missing Unreleased',
        text: missing_unreleased
    },
    {
        id: 'release-with-no-changes',
        title: 'Release with No Changes',
        text: release_with_no_changes
    },
    {
        id: 'changes-with-no-group',
        title: 'Changes with No Group',
        text: changes_with_no_group
    },
    {
        id: 'change-group-typo',
        title: 'Change Group Typo',
        text: changes_group_typo
    }
]

await main()

async function main() {
    await init()

    const exampleLinks = document.querySelector('.example-links')
    const output = document.querySelector(".output textarea")
    const errorList = document.querySelector(".errors .list")

    let newDoc = true

    function debounce(func, timeout = 300) {
        let timer;
        return (...args) => {
            clearTimeout(timer);
            timer = setTimeout(() => {
                func.apply(this, args);
            }, timeout);
        };
    }

    const editorExtensions = [
        EditorView.lineWrapping,
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightSpecialChars(),
        history(),
        drawSelection(),
        EditorState.allowMultipleSelections.of(true),
        indentOnInput(),
        syntaxHighlighting(defaultHighlightStyle, {fallback: true}),
        rectangularSelection(),
        crosshairCursor(),
        highlightActiveLine(),
        keymap.of([
            ...defaultKeymap,
            ...historyKeymap,
            ...lintKeymap
        ]),
        markdown(),
        EditorView.updateListener.of(debounce(v => {
            if (v.docChanged || newDoc) {
                newDoc = false
                output.textContent = 'Loading...'
                setTimeout(() => {
                    const input = editorView.state.doc.toString()
                    output.textContent = print_syntax_tree(input)
                }, 0)
            }
        }, 200)),
        lintGutter(),
        linter(v => {
            errorList.innerHTML = 'Loading...'
            const input = editorView.state.doc.toString()
            let errors = get_errors(input).map(error => {
                return {
                    message: error.message,
                    from: error?.start?.offset || 0,
                    to: error?.end?.offset || 0,
                    line: error?.start?.line || 0,
                    column: error?.start?.column || 0,
                    severity: 'error'
                }
            })
            if (errors.length === 0) {
                errorList.innerHTML = '✓ No Errors'
            } else {
                errorList.innerHTML = ''
                const ul = document.createElement('ul')
                errors.forEach(error => {
                    const li = document.createElement('li')
                    li.classList.add('error')
                    ul.appendChild(li)

                    const leftDiv = document.createElement('div')
                    leftDiv.classList.add('left')
                    leftDiv.textContent = `[${error.line}:${error.column}]`
                    li.appendChild(leftDiv)


                    const rightDiv = document.createElement('div')
                    rightDiv.classList.add('right')
                    li.appendChild(rightDiv)

                    const pre = document.createElement('pre')
                    rightDiv.appendChild(pre)

                    const code = document.createElement('code')
                    code.textContent = error.message
                    pre.appendChild(code)
                })
                errorList.appendChild(ul)
            }
            return errors
        })
    ]

    window.addEventListener("popstate", event => {
        if (event.state) {
            const example = findExample(document.location.hash)
            replaceDoc(example.text)
        }
    })

    function replaceDoc(text) {
        const newState = EditorState.create({
            doc: text,
            extensions: editorExtensions
        })
        editorView.setState(newState)
        newDoc = true
    }

    EXAMPLES.forEach((example, i, examples) => {
        const link = document.createElement('a')
        link.href = `#${example.id}`
        link.textContent = example.title
        link.onclick = (event) => {
            event.preventDefault()
            if (document.location.hash !== link.hash) {
                window.history.pushState(example.id, "", link.href)
            }
            replaceDoc(example.text)
        }
        exampleLinks.appendChild(link)
        if (i !== examples.length - 1) {
            const comma = document.createElement('span')
            comma.textContent = ', '
            exampleLinks.appendChild(comma)
        }
    })

    const example = findExample(document.location.hash)

    const editorView = new EditorView({
        doc: example ? example.text : spec,
        extensions: editorExtensions,
        parent: document.querySelector('.editor')
    })
}

function findExample(idOrHash) {
    const id = idOrHash.replace(/^#/, '')
    return EXAMPLES.find(example => example.id === id)
}
