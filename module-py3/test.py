# build: cargo build --release && cp ../target/release/libpybig2.so ./pybig2.so && python3 test.py
import pybig2

print (pybig2.score_hand(0xF1000))
print (pybig2.score_hand(0x1000))

def play_a_game():
    BIG2 = pybig2.GameServer(8)

    BIG2.deal(None)

    p = BIG2.turn()
    board = BIG2.board()
    #print ("Turn: player %d, board: 0x%16x" % (p, board))

    BIG2.action_play(p, 0x1000)

    p = BIG2.turn()
    board = BIG2.board()
    #print ("Turn: player %d, board: 0x%16x" % (p, board))

    BIG2.action_pass(p)

    p = BIG2.turn()
    board = BIG2.board()
    #print ("Turn: player %d, board: 0x%16x" % (p, board))

    try:
        BIG2.action_play(p, 0x2000)
    except:
        pass

    p = BIG2.turn()
    board = BIG2.board()
    #print ("Turn: player %d, board: 0x%16x" % (p, board))

    BIG2.action_pass(p)

if __name__ == "__main__":
    import timeit
    setup = "from __main__ import play_a_game"
    print (timeit.timeit("play_a_game()", setup=setup, number=62500))