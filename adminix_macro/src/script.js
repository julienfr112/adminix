document.addEventListener('click', function(e) {
    var down_class = ' dir-d '
    var up_class = ' dir-u '
    var regex_dir = / dir-(u|d) /
    var regex_table = /\bsortable\b/
    var element = e.target

    function reClassify(element, dir) {
        element.className = element.className.replace(regex_dir, '') + dir
    }

    function getValue(element) {
        return element.firstChild.value || element.innerText
    }
    console.log(element.nodeName)
    if (element.nodeName === 'TH') {
        try {
            var tr = element.parentNode
            var table = tr.parentNode.parentNode
            if (regex_table.test(table.className)) {
                var column_index
                var nodes = tr.cells
                for (var i = 0; i < nodes.length; i++) {
                    if (nodes[i] === element) {
                        column_index = i
                    } else {
                        reClassify(nodes[i], '')
                    }
                }

                var dir = down_class
                if (element.className.indexOf(down_class) !== -1) {
                    dir = up_class
                }
                reClassify(element, dir)
                var org_tbody = table.tBodies[0]
                var rows = [].slice.call(org_tbody.rows, 0)
                var reverse = dir === up_class
                rows.sort(function(a, b) {
                    var x = getValue((reverse ? a : b).cells[column_index])
                    var y = getValue((reverse ? b : a).cells[column_index])
                    return isNaN(x - y) ? x.localeCompare(y) : x - y
                })
                var clone_tbody = org_tbody.cloneNode()
                while (rows.length) {
                    clone_tbody.appendChild(rows.splice(0, 1)[0])
                }
                table.replaceChild(clone_tbody, org_tbody)
            }
        } catch (error) { console.log(error) }
    }
})