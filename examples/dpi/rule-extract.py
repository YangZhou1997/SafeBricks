
def remove_duplicates(l):
    return list(set(l))

rulesetnames = ["community-snort2.9.rules", "community-snort3.rules", "emerging-all-snort2.9.rules", "emerging-all-snort-edge.rules", "emerging-all-suricata2.0.rules", "emerging-all-suricata4.0.rules"]

if __name__ == "__main__":
    sentense_ruleset = []
    word_ruleset = []
        
    for rulesetname in rulesetnames:
        f = open("./rawrules/" + rulesetname, "r")
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
                sentense_ruleset.append(content)
                word_ruleset += content.split()
                
                content_start = content_end + len(content_end_str)
                content_start = rule.find(content_start_str, content_start) + len(content_start_str)
                content_end = rule.find(content_end_str, content_start)

                # print content_start, content_end
                # print content

        sentense_ruleset = remove_duplicates(sentense_ruleset)
        word_ruleset = remove_duplicates(word_ruleset)

        # print word_ruleset
        print "number of sentense rules " + str(len(sentense_ruleset))
        print "number of word rules " + str(len(word_ruleset))

        f.close()


    f = open("./sentenserules/sentense.rules", "w")
    for i in sentense_ruleset:
        f.write(i + "\n")
    f.close()

    f = open("./wordrules/word.rules", "w")
    for i in word_ruleset:
        f.write(i + "\n")
    f.close()

