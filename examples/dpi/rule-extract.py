
def remove_duplicates(l):
    return list(set(l))

if __name__ == "__main__":
    f = open("./snort3-community.rules", "r")
    raw_ruleset = []
    word_ruleset = []
    for rule in f:
        if "alert" not in rule:
            continue
        
        content_start_str = "content:\""
        content_end_str = "\""

        content_start = 0
        content_start = rule.find(content_start_str, content_start) + len(content_start_str)
        content_end = rule.find(content_end_str, content_start)

        while content_start != -1 + len(content_start_str):
            content = rule[content_start: content_end]
            raw_ruleset.append(content)
            word_ruleset += content.split()
            
            content_start = content_end + len(content_end_str)
            content_start = rule.find(content_start_str, content_start) + len(content_start_str)
            content_end = rule.find(content_end_str, content_start)

            # print content_start, content_end
            # print content

    word_ruleset = remove_duplicates(word_ruleset)

    # print word_ruleset

    print "number of raw_ruleset " + str(len(raw_ruleset))
    print "number of word_ruleset " + str(len(word_ruleset))

    f = open("raw.rules", "w")
    for i in raw_ruleset:
        f.write(i + "\n")
    f.close()

    f = open("word.rules", "w")
    for i in word_ruleset:
        f.write(i + "\n")
    f.close()

