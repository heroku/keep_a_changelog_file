import init, {get_errors, print_syntax_tree} from "./demo.js"
import {findSample, getSamples, defaultText} from "./samples.js";
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

async function main() {
    await init()

    const sampleLinks = document.querySelector('.sample-links')
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
            const sample = findSample(document.location.hash)
            replaceDoc(sample.text)
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

    getSamples().forEach((sample, i, samples) => {
        const link = document.createElement('a')
        link.href = `#${sample.id}`
        link.textContent = sample.title
        link.onclick = (event) => {
            event.preventDefault()
            if (document.location.hash !== link.hash) {
                window.history.pushState(sample.id, "", link.href)
            }
            replaceDoc(sample.text)
        }
        sampleLinks.appendChild(link)
        if (i !== samples.length - 1) {
            const comma = document.createElement('span')
            comma.textContent = ', '
            sampleLinks.appendChild(comma)
        }
    })

    const sample = findSample(document.location.hash)

    const editorView = new EditorView({
        doc: sample ? sample.text : defaultText(),
        extensions: editorExtensions,
        parent: document.querySelector('.editor')
    })
}

await main()
