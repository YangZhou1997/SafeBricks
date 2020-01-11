
def dedup(l):
    return list(set(l))

rulesetnames = ["community-snort2.9.rules", "community-snort3.rules", "emerging-all-snort2.9.rules", "emerging-all-snort-edge.rules", "emerging-all-suricata2.0.rules", "emerging-all-suricata4.0.rules"]

def extract(rule, ps, pe):
    contents = []
    
    s_index = 0
    s_index = rule.find(ps, s_index) + len(ps)
    e_index = rule.find(pe, s_index)

    while s_index != -1 + len(ps):
        content = rule[s_index: e_index]
        contents.append(content)
        # words.extend(content.split())
        
        s_index = e_index + len(pe)
        s_index = rule.find(ps, s_index) + len(ps)
        e_index = rule.find(pe, s_index)

        # print s_index, e_index
        # print content

    return contents

# return rule strings, rule words, and rule regex for this rule files
def get_rules(rulesetname):
    sente_rules = []
    word_rules = []
    regex_rules = []

    lines = open("./rawrules/" + rulesetname, "r").read().strip('\n').split('\n')
    for rule in lines:
        if "alert" not in rule:
            continue

        sentes = extract(rule, "content:\"", "\"")
        regexes = extract(rule, "pcre:\"", "\"")

        sente_rules.extend(sentes)
        regex_rules.extend(regexes)

        for sente in sentes:
            word_rules.extend(sente.split())
    
    # print word_ruleset
    print(f'ruleset: {rulesetname}')
    print("number of sentense rules " + str(len(sente_rules)))
    print("number of word rules " + str(len(word_rules)))
    print("number of regex rules " + str(len(regex_rules)))

    return sente_rules, word_rules, regex_rules

if __name__ == "__main__":
    sente_ruleset = []
    word_ruleset = []
    regex_ruleset = []

    for rulesetname in rulesetnames:
        sente_rules, word_rules, regex_rules = get_rules(rulesetname)
        sente_ruleset.extend(sente_rules)
        word_ruleset.extend(word_rules)
        regex_ruleset.extend(regex_rules)

    sente_ruleset = dedup(sente_ruleset)
    word_ruleset = dedup(word_ruleset)
    regex_ruleset = dedup(regex_ruleset)

    f = open("./rules/sentense.rules", "w")
    for i in sente_ruleset:
        f.write(i + "\n")
    f.close()

    f = open("./rules/word.rules", "w")
    for i in word_ruleset:
        f.write(i + "\n")
    f.close()

    f = open("./rules/regex.rules", "w")
    for i in regex_ruleset:
        f.write(i + "\n")
    f.close()

    f_sgx = open("./rules/dpirules.rs", "w")
    cnt = 0
    for i in word_ruleset:
        if cnt == 0:
            f_sgx.write("pub const DPIRULES: [&str; %d] = [r#\"" % (len(word_ruleset),) + i + "\"#, ")
        elif cnt == len(word_ruleset) - 1:
            f_sgx.write("r#\"" + i + "\"#];")
        else:
            f_sgx.write("r#\"" + i + "\"#, ")
        cnt += 1
    f_sgx.close()
