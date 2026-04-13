import sys

import json
from seagull import Key, Stroke, Outline

def load_dictionary(dict_filepath):
    dictionary = {}
    with open(dict_filepath) as main_json_fp:
        main_json = json.load(main_json_fp)

        for outline, word in main_json.items():
            try:
                outline = Outline(outline)
            except ValueError:
#                print("Couldn't add", outline, repr(word))
                continue

            dictionary[outline] = word

    return dictionary

MAIN = load_dictionary('data/main.json')

def main():
    word = sys.argv[1]

    for outline, word_ in MAIN.items():
        if word == word_:
            print(outline)


if __name__ == "__main__":
    main()
