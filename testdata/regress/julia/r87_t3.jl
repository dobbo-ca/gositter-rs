# n-queens (nqueens) solver, for nsquaresx-by-nsquaresy board

struct Queen
    x::Int
    y::Int
end
hitshorz(queena, queenb) = queena.x == queenb.x
hitsvert(queena, queenb) = queena.y == queenb.y
hitsdiag(queena, queenb) = abs(queena.x - queenb.x) == abs(queena.y - queenb.y)
hitshvd(qa, qb) = hitshorz(qa, qb) || hitsvert(qa, qb) || hitsdiag(qa, qb)
hitsany(testqueen