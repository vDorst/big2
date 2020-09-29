# build: cargo build --release && cp ../target/release/libpybig2.so ./pybig2.so && python3 test.py
import pybig2

print (pybig2.score_hand(0xF1000))
print (pybig2.score_hand(0x1000))

def play_a_game():
    BIG2 = pybig2.GameServer(8)

    BIG2.deal( [ 0x1111_1111_1111_1000, 0x2222_2222_2222_2000, 0x4444_4444_4444_4000, 0x8888_8888_8888_8000 ] )

    p = BIG2.turn()

    c = 0x1000
    
    while (p != -1):
            try:
                BIG2.action_play(p, c)
                p = BIG2.turn()
                c <<= 1
            except:
                print ("Can't play P%d, C0x%16x" % ( p, c ))
                break

    # Play a new round

    BIG2.deal( [ 0x8888_8888_8888_8000, 0x1111_1111_1111_1000, 0x2222_2222_2222_2000, 0x4444_4444_4444_4000 ] )

    p = BIG2.turn()
    # print ("Board: %x" % BIG2.board())

    c = 0x8000
    
    while (p != -1):
            try:
                BIG2.action_play(p, c)
                p = BIG2.turn()
                c <<= 1
            except:
                print ("Can't play P%d, C0x%16x" % ( p, c ))
                break
    #print ("Ende")

if __name__ == "__main__":
    import timeit
    setup = "from __main__ import play_a_game"
    print (timeit.timeit("play_a_game()", setup=setup, number=100000))