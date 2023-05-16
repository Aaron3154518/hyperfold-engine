

// Text
pub struct Text {
    start: usize,
    len: usize,
    w: u32
};

impl Text {
    pub fn draw()
}

pub fn Text::draw(TextureBuilder& tex, Rect rect, const SharedFont& font,
                const std::string& text) {
    Surface textSurf = makeSurface();
    std::string str = text.substr(start, len);
    textSurf = makeSurface(
        TTF_RenderText_Blended(font.get(), str.c_str(), Colors::Black));

    RenderData rd(makeSharedTexture(
        SDL_CreateTextureFromSurface(Renderer::get(), textSurf.get())));
    rd.mRect = rd.getMinRect(rect);
    rd.mRect.setPos(rect, Rect::Align::CENTER);
    tex.draw(rd);
}

// Line
struct Line {
   public:
    friend class TextData;
    enum Type : uint8_t {
        TEXT = 0,
        IMAGE,
    };

    bool empty() const;
    int w() const;
    int getSpace(int maxW) const;
    size_t numImgs() const;

    void addText(const Text& text);
    void addImage(int lineH);

   private:
    int mW = 0, mImgCnt = 0;
    std::vector<Type> mTypes;
    std::vector<Text> mText;
};


bool Line::empty() const { return mTypes.empty(); }

int Line::w() const { return mW; }

int Line::getSpace(int maxW) const { return maxW - mW; }

size_t Line::numImgs() const { return mImgCnt; }

pub fn Line::addText(const Text& text) {
    mW += text.w;
    mText.push_back(text);
    mTypes.push_back(Type::TEXT);
}
pub fn Line::addImage(int lineH) {
    mW += lineH;
    mImgCnt++;
    mTypes.push_back(Type::IMAGE);
}

// splitText
std::vector<Line> splitText(const std::string& text, SharedFont font,
                            int maxW) {
    std::vector<Line> lines;
    lines.push_back(Line());
    if (!font) {
        return lines;
    }

    int lineH = TTF_FontHeight(font.get());
    int spaceW;
    TTF_SizeText(font.get(), " ", &spaceW, nullptr);

    std::function<pub fn(size_t, size_t)> addText =
        [&lines, &font, &text, &addText, maxW, spaceW](size_t pos1,
                                                       size_t pos2) {
            if (pos1 >= pos2) {
                return;
            }

            size_t len = pos2 - pos1;
            std::string str = text.substr(pos1, len);
            int count = 0, width = 0;
            TTF_MeasureUTF8(font.get(), str.c_str(),
                            lines.back().getSpace(maxW), &width, &count);

            if ((size_t)count == len) {  // Fit entire text onto line
                lines.back().addText({pos1, len, width});
            } else {  // Break up text
                // Find last space, if any
                size_t lastSpace = str.find_last_of(' ');
                if (lastSpace != std::string::npos) {  // Break into words
                    int textW;
                    TTF_SizeText(font.get(), str.substr(0, lastSpace).c_str(),
                                 &textW, nullptr);

                    lines.back().addText({pos1, pos1 + lastSpace, textW});
                    lines.push_back(Line());
                    addText(pos1 + lastSpace + 1, pos2);
                } else {  // Won't fit on this line
                    // Get the length until the next break
                    int wordW = 0;
                    size_t space = str.find(' ');
                    TTF_SizeUTF8(font.get(), str.substr(0, space).c_str(),
                                 &wordW, nullptr);
                    if (wordW <= maxW) {  // It will fit on the next line
                        lines.push_back(Line());
                        addText(pos1, pos2);
                    } else {  // It is bigger than one line, split across
                        // multiple lines
                        lines.back().addText({pos1, (size_t)count, width});
                        lines.push_back(Line());
                        addText(pos1 + count, pos2);
                    }
                }
            }
        };

    std::string delims = "\n{";
    size_t pos = 0, idx = text.find_first_of(delims, pos);
    while (idx != std::string::npos) {
        addText(pos, idx);
        switch (text.at(idx)) {
            case '\n':
                lines.push_back(Line());
                break;
            case '{':
                pos = idx + 1;
                idx = text.find('}', pos);
                if (idx == std::string::npos) {
                    std::cerr << "splitText(): Unterminated '{'" << std::endl;
                    return lines;
                }
                if (idx == pos) {
                    break;
                }
                switch (text.at(pos)) {
                    case 'b':
                        break;
                    case 'i':
                        if (lineH > lines.back().getSpace(maxW)) {
                            lines.push_back(Line());
                        }
                        lines.back().addImage(lineH);
                        break;
                };
                break;
        }
        pos = idx + 1;
        idx = text.find_first_of(delims, pos);
    }
    addText(pos, text.size());

    return std::move(lines);
}